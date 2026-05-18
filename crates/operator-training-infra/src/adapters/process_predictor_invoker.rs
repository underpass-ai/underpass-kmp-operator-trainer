//! `Predictor` backed by `std::process::Command`. Spawns the
//! configured `PredictorTarget` synchronously, waits for the child
//! to exit, and surfaces the result as a typed `PredictorOutcome`.
//!
//! Command line (matching `scripts/operator/predict_operator_sft.py`):
//!
//! ```text
//! <command> --model-id <base_model> --adapter <adapter_dir> \
//!     --dataset-jsonl <dataset_path> --output <output_dir>
//! ```
//!
//! stdin is redirected to `/dev/null` for the same reason as
//! `ProcessTrainerInvoker`: the predictor must never block waiting
//! for input the parent never sends. stdout / stderr are inherited;
//! this adapter does not parse predictor logs — it reads
//! `<output_dir>/summary.json` after the child exits to populate
//! `PredictorOutcome.predictions` and `PredictorOutcome.failures`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use operator_training_application::errors::predictor_error::PredictorError;
use operator_training_application::ports::predictor::Predictor;
use operator_training_application::ports::predictor_outcome::PredictorOutcome;
use operator_training_application::ports::predictor_target::PredictorTarget;
use serde::Deserialize;

const ADAPTER: &str = "process_predictor_invoker";

#[derive(Debug, Default, Clone)]
pub struct ProcessPredictorInvoker;

impl ProcessPredictorInvoker {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct SummaryDto {
    #[serde(default)]
    predictions: usize,
    #[serde(default)]
    failures: usize,
}

impl Predictor for ProcessPredictorInvoker {
    fn predict(&self, target: &PredictorTarget) -> Result<PredictorOutcome, PredictorError> {
        let mut command = Command::new(target.command().as_str());
        command
            .stdin(Stdio::null())
            .arg("--model-id")
            .arg(target.base_model().as_str())
            .arg("--adapter")
            .arg(target.adapter_directory().as_str())
            .arg("--dataset-jsonl")
            .arg(target.dataset_path())
            .arg("--output")
            .arg(target.output_directory().as_str());
        let status = command
            .status()
            .map_err(|err| PredictorError::SpawnFailure {
                adapter: ADAPTER,
                command: target.command().as_str().to_string(),
                message: err.to_string(),
            })?;
        if !status.success() {
            return Err(PredictorError::NonZeroExit {
                adapter: ADAPTER,
                command: target.command().as_str().to_string(),
                exit_code: status.code(),
            });
        }
        let output_dir = PathBuf::from(target.output_directory().as_str());
        let predictions_path = output_dir.join("predictions.jsonl");
        let summary_path = output_dir.join("summary.json");
        let summary = read_summary(&summary_path).map_err(|err| PredictorError::SpawnFailure {
            adapter: ADAPTER,
            command: target.command().as_str().to_string(),
            message: err,
        })?;
        PredictorOutcome::new(
            predictions_path.to_string_lossy().to_string(),
            summary_path.to_string_lossy().to_string(),
            summary.predictions,
            summary.failures,
        )
        .map_err(|err| PredictorError::SpawnFailure {
            adapter: ADAPTER,
            command: target.command().as_str().to_string(),
            message: format!("build outcome: {err}"),
        })
    }
}

fn read_summary(path: &Path) -> Result<SummaryDto, String> {
    let bytes =
        fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str::<SummaryDto>(&bytes).map_err(|err| format!("parse summary.json: {err}"))
}
