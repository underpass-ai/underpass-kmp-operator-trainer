//! `operator-dpo-pair-generator` — generate DPO chosen/rejected pairs.
//!
//! This binary intentionally lives in the CLI crate, not the domain crate:
//! it reads `OpenAI` SFT JSONL, uses infra mappers to cross the wire/domain
//! boundary, and writes TRL-oriented JSONL. The semantic checks still use the
//! typed `OperatorAction` domain model and the strict contract validator.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::contract::correctness::action_correctness::ActionCorrectness;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::ports::scenario_source::ScenarioSource;
use operator_synthetic_infra::adapters::jsonl_scenario_source::JsonlScenarioSource;
use serde_json::{Map, Value, json};

#[derive(Parser, Debug)]
#[command(
    name = "operator-dpo-pair-generator",
    about = "Generate DPO chosen/rejected pairs from Operator SFT JSONL.",
    version
)]
struct Cli {
    /// SFT train JSONL produced by `prepare_operator_sft_dataset.py`.
    #[arg(long)]
    train_jsonl: PathBuf,
    /// Realistic scenarios JSONL used to reconstruct visible state.
    #[arg(long)]
    scenarios_jsonl: PathBuf,
    /// Output pairs JSONL path.
    #[arg(long)]
    output: PathBuf,
    /// Tools receiving array-copy perturbations.
    #[arg(long, default_value = "kernel_ingest,kernel_write_memory")]
    target_tools: String,
    /// Maximum persisted perturbations per row after validation.
    #[arg(long, default_value_t = 6)]
    max_per_row: usize,
    /// Deterministic seed recorded in summary. Perturbations are stable.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Add spurious extra-field perturbations across every action kind.
    ///
    /// By default, spurious fields are generated only for write tools plus
    /// stop/escalate rows. That matches the v8.1.2 DPO target and keeps the
    /// pair count in the expected range.
    #[arg(long)]
    spurious_field_all_rows: bool,
    /// Overwrite existing output files.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Clone)]
struct TrainRow {
    step_id: String,
    scenario_id: String,
    prompt_messages: Vec<Value>,
    chosen_dto: OperatorActionDto,
    chosen_json: Value,
}

#[derive(Debug, Clone)]
struct ScenarioContext {
    mode: OperatorMode,
    visible_state: VisibleState,
    about: operator_shared_domain::ids::about_id::AboutId,
}

#[derive(Debug, Clone)]
struct PerturbationCandidate {
    perturbation: PerturbationName,
    rejected_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PerturbationName {
    ReorderArray(ArrayFieldPath),
    DropElement(ArrayFieldPath),
    MutateRef(ArrayFieldPath),
    MutateKind(ArrayFieldPath),
    SpuriousExtraField(SpuriousFieldKind),
}

impl PerturbationName {
    fn name(&self) -> &'static str {
        match self {
            Self::ReorderArray(_) => "reorder_array",
            Self::DropElement(_) => "drop_element",
            Self::MutateRef(_) => "mutate_ref",
            Self::MutateKind(_) => "mutate_kind",
            Self::SpuriousExtraField(_) => "spurious_extra_field",
        }
    }

    fn field(&self) -> &'static str {
        match self {
            Self::ReorderArray(field)
            | Self::DropElement(field)
            | Self::MutateRef(field)
            | Self::MutateKind(field) => field.as_str(),
            Self::SpuriousExtraField(field) => field.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ArrayFieldPath {
    MemoryEntries,
    MemoryRelations,
    MemoryEvidence,
    MemoryDimensions,
    Related,
}

impl ArrayFieldPath {
    fn as_str(self) -> &'static str {
        match self {
            Self::MemoryEntries => "memory.entries[*]",
            Self::MemoryRelations => "memory.relations[*]",
            Self::MemoryEvidence => "memory.evidence[*]",
            Self::MemoryDimensions => "memory.dimensions[*]",
            Self::Related => "related[*]",
        }
    }

    fn json_path(self) -> &'static [&'static str] {
        match self {
            Self::MemoryEntries => &["arguments", "memory", "entries"],
            Self::MemoryRelations => &["arguments", "memory", "relations"],
            Self::MemoryEvidence => &["arguments", "memory", "evidence"],
            Self::MemoryDimensions => &["arguments", "memory", "dimensions"],
            Self::Related => &["arguments", "related"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SpuriousFieldKind {
    Evidence,
    Arguments,
}

impl SpuriousFieldKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Evidence => "evidence",
            Self::Arguments => "arguments",
        }
    }
}

#[derive(Debug, Default)]
struct GenerationStats {
    persisted_by_perturbation: BTreeMap<String, usize>,
    persisted_by_tool: BTreeMap<String, usize>,
    capped_by_reason: BTreeMap<String, usize>,
    skipped_by_reason: BTreeMap<String, usize>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("dpo-pair-generator failed: {err}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(cli: &Cli) -> Result<(), String> {
    precheck(cli)?;
    let target_tools = parse_target_tools(&cli.target_tools)?;
    let scenarios = load_scenario_contexts(&cli.scenarios_jsonl)?;
    let rows = read_train_rows(&cli.train_jsonl)?;
    let output_dir = cli.output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_dir)
        .map_err(|err| format!("create output dir {}: {err}", output_dir.display()))?;
    let skipped_path = output_dir.join("skipped_perturbations.jsonl");
    let summary_path = output_dir.join("summary.json");
    let mut pairs_writer = File::create(&cli.output)
        .map_err(|err| format!("create --output {}: {err}", cli.output.display()))?;
    let mut skipped_writer = File::create(&skipped_path)
        .map_err(|err| format!("create skipped file {}: {err}", skipped_path.display()))?;
    let validator = CompositeActionContractValidator::default_strict();
    let mut stats = GenerationStats::default();
    let mut pair_count = 0usize;

    for row in &rows {
        let Some(context) = scenarios.get(&row.scenario_id) else {
            write_skip(
                &mut skipped_writer,
                &mut stats,
                row,
                None,
                "scenario_not_found",
            )?;
            continue;
        };
        let chosen_action = match OperatorActionMapper::to_domain(&row.chosen_dto) {
            Ok(action) => action,
            Err(err) => {
                write_skip(
                    &mut skipped_writer,
                    &mut stats,
                    row,
                    None,
                    &format!("chosen_shape_invalid:{err}"),
                )?;
                continue;
            }
        };
        if let Err(violations) = validator.validate(
            &chosen_action,
            &context.about,
            context.mode,
            &context.visible_state,
        ) {
            write_skip(
                &mut skipped_writer,
                &mut stats,
                row,
                None,
                &format!("chosen_contract_invalid:{}", violation_codes(&violations)),
            )?;
            continue;
        }

        let candidates = candidates_for_row(
            row,
            &chosen_action,
            &context.visible_state,
            &target_tools,
            cli.spurious_field_all_rows,
        );
        let mut valid_candidates = Vec::new();
        for candidate in candidates {
            let validation = validate_rejected(
                &candidate.rejected_json,
                &chosen_action,
                context,
                &validator,
            );
            let rejected_violation_codes = match validation {
                CandidateValidation::Keep { violation_codes } => violation_codes,
                CandidateValidation::Skip { reason } => {
                    write_skip(
                        &mut skipped_writer,
                        &mut stats,
                        row,
                        Some(&candidate.perturbation),
                        &reason,
                    )?;
                    continue;
                }
            };
            if canonical_json(&row.chosen_json) == canonical_json(&candidate.rejected_json) {
                write_skip(
                    &mut skipped_writer,
                    &mut stats,
                    row,
                    Some(&candidate.perturbation),
                    "chosen_equals_rejected",
                )?;
                continue;
            }
            valid_candidates.push((candidate, rejected_violation_codes));
        }

        for (index, (candidate, rejected_violation_codes)) in
            valid_candidates.into_iter().enumerate()
        {
            if index >= cli.max_per_row {
                increment(&mut stats.capped_by_reason, "max_per_row_reached");
                continue;
            }
            let pair = pair_json(row, &candidate, &rejected_violation_codes);
            writeln!(pairs_writer, "{pair}")
                .map_err(|err| format!("write {}: {err}", cli.output.display()))?;
            pair_count += 1;
            increment(
                &mut stats.persisted_by_perturbation,
                candidate.perturbation.name(),
            );
            increment(&mut stats.persisted_by_tool, chosen_label(&row.chosen_dto));
        }
    }

    let summary = json!({
        "event": "kernel_operator_dpo_pair_generator.completed",
        "train_jsonl": cli.train_jsonl,
        "scenarios_jsonl": cli.scenarios_jsonl,
        "output": cli.output,
        "skipped_output": skipped_path,
        "seed": cli.seed,
        "rows": rows.len(),
        "pairs": pair_count,
        "target_tools": target_tools.iter().map(|tool| tool.as_str()).collect::<Vec<_>>(),
        "max_per_row": cli.max_per_row,
        "spurious_field_all_rows": cli.spurious_field_all_rows,
        "persisted_by_perturbation": stats.persisted_by_perturbation,
        "persisted_by_tool": stats.persisted_by_tool,
        "capped_by_reason": stats.capped_by_reason,
        "skipped_by_reason": stats.skipped_by_reason,
    });
    fs::write(
        &summary_path,
        serde_json::to_string_pretty(&summary).expect("summary serializes") + "\n",
    )
    .map_err(|err| format!("write {}: {err}", summary_path.display()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("summary serializes")
    );
    Ok(())
}

fn precheck(cli: &Cli) -> Result<(), String> {
    require_readable_non_empty(&cli.train_jsonl, "--train-jsonl")?;
    require_readable_non_empty(&cli.scenarios_jsonl, "--scenarios-jsonl")?;
    if cli.output.exists() && !cli.force {
        return Err(format!(
            "--output {} already exists; pass --force",
            cli.output.display()
        ));
    }
    if cli.max_per_row == 0 {
        return Err("--max-per-row must be > 0".to_string());
    }
    Ok(())
}

fn require_readable_non_empty(path: &Path, label: &str) -> Result<(), String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("{label} {}: {err}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{label} {} is not a file", path.display()));
    }
    if metadata.len() == 0 {
        return Err(format!("{label} {} is empty", path.display()));
    }
    Ok(())
}

fn parse_target_tools(raw: &str) -> Result<BTreeSet<KernelTool>, String> {
    let mut tools = BTreeSet::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        tools.insert(KernelTool::parse(trimmed).map_err(|err| err.to_string())?);
    }
    if tools.is_empty() {
        return Err("--target-tools must include at least one tool".to_string());
    }
    Ok(tools)
}

fn load_scenario_contexts(path: &Path) -> Result<BTreeMap<String, ScenarioContext>, String> {
    let scenarios = JsonlScenarioSource::new(path)
        .read()
        .map_err(|err| format!("read scenarios {}: {err}", path.display()))?;
    let mut out = BTreeMap::new();
    for scenario in scenarios {
        out.insert(
            scenario.id().as_str().to_string(),
            scenario_context(&scenario),
        );
    }
    Ok(out)
}

fn scenario_context(scenario: &Scenario) -> ScenarioContext {
    ScenarioContext {
        mode: scenario.subject().mode(),
        visible_state: scenario.subject().visible_state().clone(),
        about: scenario.subject().about().clone(),
    }
}

fn read_train_rows(path: &Path) -> Result<Vec<TrainRow>, String> {
    let file = File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for (zero_based, line) in reader.lines().enumerate() {
        let line_no = zero_based + 1;
        let line = line.map_err(|err| format!("{}:{line_no}: {err}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|err| format!("{}:{line_no}: invalid JSON: {err}", path.display()))?;
        rows.push(parse_train_row(&value, path, line_no)?);
    }
    if rows.is_empty() {
        return Err(format!("{} contained no train rows", path.display()));
    }
    Ok(rows)
}

fn parse_train_row(value: &Value, path: &Path, line_no: usize) -> Result<TrainRow, String> {
    let step_id = value
        .get("step_id")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{}:{line_no}: missing step_id", path.display()))?
        .to_string();
    let scenario_id = scenario_id_from_step_id(&step_id);
    let messages = value
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}:{line_no}: missing messages array", path.display()))?;
    if messages.len() != 3 {
        return Err(format!(
            "{}:{line_no}: expected 3 messages, got {}",
            path.display(),
            messages.len()
        ));
    }
    let assistant_content = messages[2]
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{}:{line_no}: assistant content missing", path.display()))?;
    let envelope: Value = serde_json::from_str(assistant_content).map_err(|err| {
        format!(
            "{}:{line_no}: assistant content invalid JSON: {err}",
            path.display()
        )
    })?;
    let chosen_json = envelope
        .get("action")
        .ok_or_else(|| {
            format!(
                "{}:{line_no}: assistant content missing action",
                path.display()
            )
        })?
        .clone();
    let chosen_dto: OperatorActionDto =
        serde_json::from_value(chosen_json.clone()).map_err(|err| {
            format!(
                "{}:{line_no}: action DTO parse failed: {err}",
                path.display()
            )
        })?;
    Ok(TrainRow {
        step_id,
        scenario_id,
        prompt_messages: vec![messages[0].clone(), messages[1].clone()],
        chosen_dto,
        chosen_json,
    })
}

fn scenario_id_from_step_id(step_id: &str) -> String {
    step_id
        .split_once(":step:")
        .map_or_else(|| step_id.to_string(), |(scenario, _)| scenario.to_string())
}

fn candidates_for_row(
    row: &TrainRow,
    chosen_action: &OperatorAction,
    visible_state: &VisibleState,
    target_tools: &BTreeSet<KernelTool>,
    spurious_field_all_rows: bool,
) -> Vec<PerturbationCandidate> {
    let mut candidates = Vec::new();
    let mut is_target_write_tool = false;
    if let Some(tool) = chosen_action.tool()
        && target_tools.contains(&tool)
    {
        is_target_write_tool = true;
        for field in array_fields_for_tool(tool) {
            candidates.extend(array_candidates(&row.chosen_json, field, visible_state));
        }
    }
    let is_terminal = matches!(
        chosen_action,
        OperatorAction::Stop(_) | OperatorAction::Escalate(_)
    );
    if (spurious_field_all_rows || is_target_write_tool || is_terminal)
        && let Some(candidate) = spurious_extra_field_candidate(&row.chosen_json)
    {
        candidates.push(candidate);
    }
    candidates
}

fn array_fields_for_tool(tool: KernelTool) -> Vec<ArrayFieldPath> {
    match tool {
        KernelTool::Ingest => vec![
            ArrayFieldPath::MemoryEntries,
            ArrayFieldPath::MemoryRelations,
            ArrayFieldPath::MemoryEvidence,
            ArrayFieldPath::MemoryDimensions,
        ],
        KernelTool::WriteMemory => vec![ArrayFieldPath::Related],
        _ => Vec::new(),
    }
}

fn array_candidates(
    chosen_json: &Value,
    field: ArrayFieldPath,
    visible_state: &VisibleState,
) -> Vec<PerturbationCandidate> {
    let mut out = Vec::new();
    if let Some(candidate) = drop_element_candidate(chosen_json, field) {
        out.push(candidate);
    }
    if let Some(candidate) = reorder_array_candidate(chosen_json, field) {
        out.push(candidate);
    }
    if let Some(candidate) = mutate_ref_candidate(chosen_json, field, visible_state) {
        out.push(candidate);
    }
    if let Some(candidate) = mutate_kind_candidate(chosen_json, field) {
        out.push(candidate);
    }
    out
}

fn drop_element_candidate(
    chosen_json: &Value,
    field: ArrayFieldPath,
) -> Option<PerturbationCandidate> {
    let array = get_array(chosen_json, field)?;
    if array.is_empty() {
        return None;
    }
    let mut rejected = chosen_json.clone();
    set_array(&mut rejected, field, array[1..].to_vec())?;
    Some(PerturbationCandidate {
        perturbation: PerturbationName::DropElement(field),
        rejected_json: rejected,
    })
}

fn reorder_array_candidate(
    chosen_json: &Value,
    field: ArrayFieldPath,
) -> Option<PerturbationCandidate> {
    let array = get_array(chosen_json, field)?;
    if array.len() < 2 {
        return None;
    }
    let mut reordered = array.to_vec();
    let last = reordered.len() - 1;
    reordered.swap(0, last);
    let mut rejected = chosen_json.clone();
    set_array(&mut rejected, field, reordered)?;
    Some(PerturbationCandidate {
        perturbation: PerturbationName::ReorderArray(field),
        rejected_json: rejected,
    })
}

fn mutate_ref_candidate(
    chosen_json: &Value,
    field: ArrayFieldPath,
    visible_state: &VisibleState,
) -> Option<PerturbationCandidate> {
    let visible_refs = visible_refs(visible_state);
    if visible_refs.is_empty() {
        return None;
    }
    let mut rejected = chosen_json.clone();
    let target = get_path_mut(&mut rejected, field.json_path())?;
    if replace_first_ref(target, &visible_refs) {
        Some(PerturbationCandidate {
            perturbation: PerturbationName::MutateRef(field),
            rejected_json: rejected,
        })
    } else {
        None
    }
}

fn mutate_kind_candidate(
    chosen_json: &Value,
    field: ArrayFieldPath,
) -> Option<PerturbationCandidate> {
    let mut rejected = chosen_json.clone();
    let target = get_path_mut(&mut rejected, field.json_path())?;
    if replace_first_kind(target) {
        Some(PerturbationCandidate {
            perturbation: PerturbationName::MutateKind(field),
            rejected_json: rejected,
        })
    } else {
        None
    }
}

fn spurious_extra_field_candidate(chosen_json: &Value) -> Option<PerturbationCandidate> {
    let mut rejected = chosen_json.clone();
    let object = rejected.as_object_mut()?;
    match object.get("kind").and_then(Value::as_str) {
        Some("tool_call") => {
            if object.contains_key("evidence") {
                return None;
            }
            object.insert(
                "evidence".to_string(),
                Value::Array(vec![Value::String("ref:spurious-extra-field".to_string())]),
            );
            Some(PerturbationCandidate {
                perturbation: PerturbationName::SpuriousExtraField(SpuriousFieldKind::Evidence),
                rejected_json: rejected,
            })
        }
        Some("stop" | "escalate") => {
            if object.contains_key("arguments") {
                return None;
            }
            object.insert("arguments".to_string(), Value::Object(Map::new()));
            Some(PerturbationCandidate {
                perturbation: PerturbationName::SpuriousExtraField(SpuriousFieldKind::Arguments),
                rejected_json: rejected,
            })
        }
        _ => None,
    }
}

fn get_array(chosen_json: &Value, field: ArrayFieldPath) -> Option<&[Value]> {
    get_path(chosen_json, field.json_path())?
        .as_array()
        .map(Vec::as_slice)
}

fn get_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn get_path_mut<'a>(value: &'a mut Value, path: &[&str]) -> Option<&'a mut Value> {
    let mut current = value;
    for key in path {
        current = current.get_mut(*key)?;
    }
    Some(current)
}

fn set_array(rejected: &mut Value, field: ArrayFieldPath, value: Vec<Value>) -> Option<()> {
    *get_path_mut(rejected, field.json_path())? = Value::Array(value);
    Some(())
}

fn visible_refs(visible_state: &VisibleState) -> Vec<String> {
    visible_state
        .known_refs()
        .iter()
        .map(|item| item.as_str().to_string())
        .collect()
}

fn replace_first_ref(value: &mut Value, visible_refs: &[String]) -> bool {
    match value {
        Value::String(current) if looks_like_ref(current) => {
            if let Some(replacement) = visible_refs.iter().find(|candidate| *candidate != current) {
                *current = replacement.clone();
                true
            } else {
                false
            }
        }
        Value::Array(items) => {
            for item in items {
                if replace_first_ref(item, visible_refs) {
                    return true;
                }
            }
            false
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                if replace_first_ref(item, visible_refs) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn looks_like_ref(value: &str) -> bool {
    value.contains(":node:") || value.starts_with("ref:")
}

fn replace_first_kind(value: &mut Value) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(kind)) = map.get_mut("kind") {
                *kind = alternate_kind(kind);
                return true;
            }
            for item in map.values_mut() {
                if replace_first_kind(item) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => {
            for item in items {
                if replace_first_kind(item) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn alternate_kind(current: &str) -> String {
    for candidate in [
        "decision",
        "observation",
        "constraint",
        "hypothesis",
        "task",
        "agent",
        "session",
        "attempt",
    ] {
        if candidate != current {
            return candidate.to_string();
        }
    }
    format!("{current}_mutated")
}

enum CandidateValidation {
    Keep { violation_codes: Vec<String> },
    Skip { reason: String },
}

fn validate_rejected(
    rejected_json: &Value,
    chosen_action: &OperatorAction,
    context: &ScenarioContext,
    validator: &CompositeActionContractValidator,
) -> CandidateValidation {
    if is_spurious_extra_field(rejected_json) {
        return CandidateValidation::Keep {
            violation_codes: vec!["wire_shape_extra_field".to_string()],
        };
    }
    let rejected_dto: OperatorActionDto = match serde_json::from_value(rejected_json.clone()) {
        Ok(dto) => dto,
        Err(err) => {
            return CandidateValidation::Skip {
                reason: format!("rejected_dto_invalid:{err}"),
            };
        }
    };
    let rejected_action = match OperatorActionMapper::to_domain(&rejected_dto) {
        Ok(action) => action,
        Err(err) => {
            return CandidateValidation::Skip {
                reason: format!("rejected_shape_invalid:{err}"),
            };
        }
    };
    if rejected_action == *chosen_action {
        return CandidateValidation::Skip {
            reason: "rejected_maps_to_chosen".to_string(),
        };
    }
    if let Err(violations) = validator.validate(
        &rejected_action,
        &context.about,
        context.mode,
        &context.visible_state,
    ) {
        return CandidateValidation::Keep {
            violation_codes: violations
                .as_slice()
                .iter()
                .map(|item| item.code().as_str().to_string())
                .collect(),
        };
    }
    let correctness = rejected_action.evaluate_correctness(chosen_action);
    if !correctness.is_correct() {
        return CandidateValidation::Keep {
            violation_codes: correctness
                .failed_fields()
                .map(|field| format!("action_correctness:{}", field.field_path().as_str()))
                .collect(),
        };
    }
    CandidateValidation::Skip {
        reason: "rejected_still_correct".to_string(),
    }
}

fn is_spurious_extra_field(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| match object.get("kind").and_then(Value::as_str) {
            Some("tool_call") => object.contains_key("evidence"),
            Some("stop" | "escalate") => object.contains_key("arguments"),
            _ => false,
        })
}

fn pair_json(
    row: &TrainRow,
    candidate: &PerturbationCandidate,
    rejected_violation_codes: &[String],
) -> String {
    let value = json!({
        "scenario_id": row.scenario_id,
        "step_id": row.step_id,
        "prompt_messages": row.prompt_messages,
        "chosen": row.chosen_json,
        "rejected": candidate.rejected_json,
        "perturbation": {
            "name": candidate.perturbation.name(),
            "field": candidate.perturbation.field(),
        },
        "rejected_violation_codes": rejected_violation_codes,
    });
    serde_json::to_string(&value).expect("pair serializes")
}

fn write_skip(
    writer: &mut File,
    stats: &mut GenerationStats,
    row: &TrainRow,
    perturbation: Option<&PerturbationName>,
    reason: &str,
) -> Result<(), String> {
    increment(&mut stats.skipped_by_reason, reason);
    let value = json!({
        "scenario_id": row.scenario_id,
        "step_id": row.step_id,
        "perturbation": perturbation.map(|item| json!({
            "name": item.name(),
            "field": item.field(),
        })),
        "reason": reason,
    });
    writeln!(
        writer,
        "{}",
        serde_json::to_string(&value).expect("skip serializes")
    )
    .map_err(|err| format!("write skipped perturbation: {err}"))
}

fn violation_codes(
    violations: &operator_shared_domain::contract::contract_violations::ContractViolations,
) -> String {
    violations
        .as_slice()
        .iter()
        .map(|item| item.code().as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn chosen_label(action: &OperatorActionDto) -> &'static str {
    match action {
        OperatorActionDto::ToolCall(call) => match call.arguments.tool.as_str() {
            "kernel_ingest" => "kernel_ingest",
            "kernel_wake" => "kernel_wake",
            "kernel_ask" => "kernel_ask",
            "kernel_near" => "kernel_near",
            "kernel_goto" => "kernel_goto",
            "kernel_rewind" => "kernel_rewind",
            "kernel_forward" => "kernel_forward",
            "kernel_trace" => "kernel_trace",
            "kernel_inspect" => "kernel_inspect",
            "kernel_write_memory" => "kernel_write_memory",
            _ => "unknown_tool",
        },
        OperatorActionDto::Stop(_) => "stop",
        OperatorActionDto::Escalate(_) => "escalate",
    }
}

fn increment(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).expect("canonical JSON serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_contract::tool_arguments_dto::ToolArgumentsDto;
    use operator_shared_contract::tool_call_action_dto::ToolCallActionDto;

    fn ingest_action() -> Value {
        json!({
            "kind": "tool_call",
            "tool": "kernel_ingest",
            "arguments": {
                "about": "about:test",
                "memory": {
                    "dimensions": [
                        {"id": "task:one", "kind": "task"},
                        {"id": "task:two", "kind": "task"}
                    ],
                    "entries": [
                        {
                            "id": "about:test:node:a",
                            "kind": "decision",
                            "text": "A",
                            "coordinates": [{"dimension": "task:one", "scope_id": "case", "sequence": 1}]
                        },
                        {
                            "id": "about:test:node:b",
                            "kind": "observation",
                            "text": "B",
                            "coordinates": [{"dimension": "task:two", "scope_id": "case", "sequence": 2}]
                        }
                    ],
                    "relations": [
                        {
                            "from": "about:test:node:a",
                            "to": "about:test:node:b",
                            "rel": "chosen_because",
                            "class": "causal",
                            "why": "because",
                            "confidence": "high"
                        }
                    ],
                    "evidence": [
                        {"id": "ev1", "supports": ["about:test:node:a"], "text": "evidence one"},
                        {"id": "ev2", "supports": ["about:test:node:b"], "text": "evidence two"}
                    ]
                },
                "provenance": {
                    "source_kind": "agent",
                    "source_agent": "tester",
                    "observed_at": "2026-05-24T00:00:00Z"
                },
                "idempotency_key": "idem-1",
                "dry_run": true
            }
        })
    }

    #[test]
    fn reorder_array_changes_order() {
        let candidate =
            reorder_array_candidate(&ingest_action(), ArrayFieldPath::MemoryEntries).unwrap();
        assert_eq!(candidate.perturbation.name(), "reorder_array");
        let entries = get_array(&candidate.rejected_json, ArrayFieldPath::MemoryEntries).unwrap();
        assert_eq!(entries[0]["id"], json!("about:test:node:b"));
    }

    #[test]
    fn drop_element_changes_array_length() {
        let candidate =
            drop_element_candidate(&ingest_action(), ArrayFieldPath::MemoryEvidence).unwrap();
        let evidence = get_array(&candidate.rejected_json, ArrayFieldPath::MemoryEvidence).unwrap();
        assert_eq!(evidence.len(), 1);
    }

    #[test]
    fn mutate_kind_changes_first_kind() {
        let candidate =
            mutate_kind_candidate(&ingest_action(), ArrayFieldPath::MemoryEntries).unwrap();
        let entries = get_array(&candidate.rejected_json, ArrayFieldPath::MemoryEntries).unwrap();
        assert_eq!(entries[0]["kind"], json!("observation"));
    }

    #[test]
    fn spurious_extra_field_adds_evidence_to_tool_call() {
        let candidate = spurious_extra_field_candidate(&ingest_action()).unwrap();
        assert_eq!(candidate.perturbation.name(), "spurious_extra_field");
        assert!(candidate.rejected_json.get("evidence").is_some());
    }

    #[test]
    fn tool_call_dto_round_trip_still_uses_typed_contract_shape() {
        let dto = OperatorActionDto::ToolCall(ToolCallActionDto {
            arguments: ToolArgumentsDto {
                tool: "kernel_ingest".to_string(),
                arguments: ingest_action()["arguments"].clone(),
            },
        });
        assert!(OperatorActionMapper::to_domain(&dto).is_ok());
    }
}
