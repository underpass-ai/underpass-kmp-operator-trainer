#!/usr/bin/env python3
"""Build deterministic v7 realistic Operator scenarios.

The output is external runtime data, not training data by itself. Each JSONL
row is a ScenarioDto consumed by operator-realistic-corpus. Templates are kept
inline because they are code-shaped: target, subject shape and variation knobs
must be reviewed together.
"""

from __future__ import annotations

import argparse
import json
import random
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


READ_TOOLS = [
    "kernel_wake",
    "kernel_ask",
    "kernel_near",
    "kernel_goto",
    "kernel_rewind",
    "kernel_forward",
    "kernel_trace",
    "kernel_inspect",
]

WRITE_TOOLS = ["kernel_ingest", "kernel_write_memory"]
WRITER_PRE_READ_TOOLS = [
    "kernel_wake",
    "kernel_ask",
    "kernel_near",
    "kernel_inspect",
]
FULL_TOOLS = [
    "kernel_ingest",
    "kernel_wake",
    "kernel_ask",
    "kernel_near",
    "kernel_goto",
    "kernel_rewind",
    "kernel_forward",
    "kernel_trace",
    "kernel_inspect",
    "kernel_write_memory",
]

DEFAULT_SCENARIO_COUNT = 1650

TARGETS = [
    "kernel_wake",
    "kernel_ask",
    "kernel_near",
    "kernel_goto",
    "kernel_rewind",
    "kernel_forward",
    "kernel_trace",
    "kernel_inspect",
    "kernel_ingest",
    "kernel_write_memory",
    "stop",
    "escalate",
]

ABOUT_PREFIXES_BY_THEME = {
    "technical_incident": [
        "incident:payments-timeout",
        "incident:auth-cascade",
        "incident:search-latency",
        "incident:checkout-errors",
        "incident:cache-invalidation",
    ],
    "software_migration": [
        "migration:postgres-13-to-15",
        "migration:rest-to-grpc",
        "migration:legacy-cache-removal",
        "migration:monolith-split",
        "migration:kafka-retention",
    ],
    "bug_investigation": [
        "bug:ios-login-loop",
        "bug:safari-cookie-drift",
        "bug:worker-retry-storm",
        "bug:timezone-reporting",
        "bug:webhook-duplication",
    ],
    "product_planning": [
        "product:checkout-v2",
        "product:onboarding-flow",
        "product:enterprise-audit",
        "product:pricing-rollout",
        "product:admin-console",
    ],
    "smart_writing_session": [
        "docs:rfc-write-up",
        "docs:incident-postmortem",
        "docs:architecture-decision",
        "docs:customer-brief",
        "docs:release-notes",
    ],
}

BUDGET_VARIANTS = [(4, 2000), (2, 1200), (1, 600), (8, 4000)]

REF_VOCAB = [
    "evidence",
    "decision",
    "rollback",
    "fix",
    "hypothesis",
    "constraint",
    "root-cause",
    "symptom",
    "metric",
    "trace",
    "deployment",
    "owner",
    "risk",
    "mitigation",
    "customer-impact",
    "timeline",
    "state",
    "candidate",
    "observation",
    "follow-up",
    "plan",
    "resolution",
]

DIM_VOCAB = [
    "agent:triage",
    "agent:solver",
    "agent:writer",
    "agent:reviewer",
    "session:nightly",
    "session:handoff",
    "task:diagnosis",
    "task:rollback",
    "topic:customer",
    "topic:infra",
    "attempt:primary",
    "attempt:fallback",
]


@dataclass(frozen=True)
class Template:
    target: str
    theme: str
    slug: str
    category: str
    goal: str
    mode: str | None = None


def t(
    target: str,
    theme: str,
    slug: str,
    category: str,
    goal: str,
    mode: str | None = None,
) -> Template:
    return Template(
        target=target,
        theme=theme,
        slug=slug,
        category=category,
        goal=goal,
        mode=mode,
    )


TEMPLATES_BY_TARGET: dict[str, list[Template]] = {
    "kernel_wake": [
        t(
            "kernel_wake",
            "technical_incident",
            "current-about",
            "happy",
            "Incident {about} has no current_ref or evidence visible; bounded investigation cannot start until memory is bootstrapped.",
        ),
        t(
            "kernel_wake",
            "software_migration",
            "prior-context",
            "happy",
            "Migration {about} is referenced, but no prior planning state is in current memory. Recovering that context is prerequisite.",
        ),
        t(
            "kernel_wake",
            "smart_writing_session",
            "before-write",
            "happy",
            "A write to {about} is contemplated, but existing memory for that about is not visible yet. Context must load first.",
        ),
        t(
            "kernel_wake",
            "product_planning",
            "after-stop-review",
            "adversarial",
            "Earlier stop left ambiguity; the about needs reloading to verify whether the conclusion still holds. Budget allows one retry.",
        ),
        t(
            "kernel_wake",
            "bug_investigation",
            "already-loaded-check",
            "adversarial",
            "Bug {about} is the target, but no current_ref proves memory was bootstrapped. Loading the about is the safe start.",
        ),
    ],
    "kernel_ask": [
        t(
            "kernel_ask",
            "technical_incident",
            "evidence-query",
            "happy",
            "Symptoms point to a deterministic fact about {ref_0}; no single visible node is decisive, so a bounded memory question is needed.",
        ),
        t(
            "kernel_ask",
            "product_planning",
            "clarify-choice",
            "happy",
            "Two candidate decisions are visible; neither node contains the answer directly, so a bounded planning question must disambiguate them.",
        ),
        t(
            "kernel_ask",
            "bug_investigation",
            "narrow-fact",
            "happy",
            "{ref_0} and {ref_1} appear unrelated in the summary; the missing link requires a narrow memory question.",
        ),
        t(
            "kernel_ask",
            "software_migration",
            "missing-constraint",
            "happy",
            "Migration plan is visible, but the active constraint is not on any visible node. A bounded query would resolve the decision.",
        ),
    ],
    "kernel_near": [
        t(
            "kernel_near",
            "technical_incident",
            "relations",
            "happy",
            "Anchor {ref_0} is visible in dimension {dim_0}, but its local neighborhood of relations and adjacent nodes is not expanded.",
        ),
        t(
            "kernel_near",
            "bug_investigation",
            "candidate-refs",
            "happy",
            "Writer needs candidate evidence; likely targets live near visible anchor {ref_0} in dimension {dim_0}.",
        ),
        t(
            "kernel_near",
            "product_planning",
            "ambiguous-anchor",
            "adversarial",
            "Anchor {ref_0} is visible in dimension {dim_0}; tempting to use a different anchor, but local expansion must use the visible one.",
        ),
        t(
            "kernel_near",
            "software_migration",
            "limit-pressure",
            "happy",
            "Budget is bounded; local expansion around {ref_0} in dimension {dim_0} is cheaper than broad retrieval.",
        ),
        t(
            "kernel_near",
            "smart_writing_session",
            "dimension-filter",
            "happy",
            "Visible anchor {ref_0} has relevant neighbors in dimension {dim_0}; unrelated dimensions would add noise.",
        ),
    ],
    "kernel_goto": [
        t(
            "kernel_goto",
            "technical_incident",
            "ref",
            "happy",
            "Visible refs include {ref_0}; navigating to that ref's full view is the next bounded step.",
        ),
        t(
            "kernel_goto",
            "software_migration",
            "temporal-cursor",
            "happy",
            "Earlier migration event {ref_0} is referenced by the current summary; the process needs that anchor's full view.",
        ),
        t(
            "kernel_goto",
            "bug_investigation",
            "trace-cursor",
            "happy",
            "Previous trace context points to endpoint {ref_1}; resuming at that endpoint preserves the chain.",
        ),
        t(
            "kernel_goto",
            "product_planning",
            "invented-ref-temptation",
            "adversarial",
            "Visible refs include only {ref_0}; tempting to navigate elsewhere, but that ref is the only safe destination.",
        ),
        t(
            "kernel_goto",
            "smart_writing_session",
            "cross-about",
            "adversarial",
            "Tempting to jump across abouts, but the same-about visible ref {ref_0} is the only valid target.",
        ),
    ],
    "kernel_rewind": [
        t(
            "kernel_rewind",
            "technical_incident",
            "prior-state",
            "happy",
            "Current state is visible, but its assumptions depend on a prior temporal slice not in current view.",
        ),
        t(
            "kernel_rewind",
            "bug_investigation",
            "verification",
            "happy",
            "Recent decision {ref_0} is visible; the evidence that motivated it lived earlier in the timeline.",
        ),
        t(
            "kernel_rewind",
            "software_migration",
            "missing-anchor",
            "adversarial",
            "Tempting to use a fresh anchor, but only the active temporal cursor is the safe pivot for the rewind.",
        ),
        t(
            "kernel_rewind",
            "product_planning",
            "refute",
            "happy",
            "Visible plan {ref_1} may be wrong; checking what was known before it would verify the assumption.",
        ),
        t(
            "kernel_rewind",
            "smart_writing_session",
            "multi-step",
            "happy",
            "Prepared write needs earlier read context; the active temporal cursor points to the relevant prior slice.",
        ),
    ],
    "kernel_forward": [
        t(
            "kernel_forward",
            "technical_incident",
            "current-state",
            "happy",
            "Earlier state was inspected; the current state after that point is needed to classify the incident outcome.",
        ),
        t(
            "kernel_forward",
            "software_migration",
            "after-rewind",
            "happy",
            "The process rewound to {ref_0}; the next state after that point explains what changed.",
        ),
        t(
            "kernel_forward",
            "bug_investigation",
            "adversarial-anchor",
            "adversarial",
            "Tempting to alter the anchor, but the visible active cursor already defines the forward path.",
        ),
        t(
            "kernel_forward",
            "product_planning",
            "next-event",
            "happy",
            "A planning checkpoint is visible; the following event determines whether the decision still holds.",
        ),
        t(
            "kernel_forward",
            "smart_writing_session",
            "across-sessions",
            "happy",
            "Current session context is incomplete; the next temporal slice carries the handoff continuation.",
        ),
    ],
    "kernel_trace": [
        t(
            "kernel_trace",
            "technical_incident",
            "causal",
            "happy",
            "Decision {ref_0} and final state {ref_1} are visible; the causal chain between them is not.",
        ),
        t(
            "kernel_trace",
            "bug_investigation",
            "contradiction",
            "happy",
            "Visible nodes {ref_0} and {ref_1} contradict each other; the path that produced the contradiction is hidden.",
        ),
        t(
            "kernel_trace",
            "software_migration",
            "supersession",
            "happy",
            "Stale plan {ref_0} and final plan {ref_1} are visible; the supersession relation needs reconstruction.",
        ),
        t(
            "kernel_trace",
            "smart_writing_session",
            "continued-page",
            "happy",
            "A previous trace between {ref_0} and {ref_1} returned more hops; the remaining chain is needed.",
        ),
    ],
    "kernel_inspect": [
        t(
            "kernel_inspect",
            "technical_incident",
            "after-near",
            "happy",
            "Near expansion returned candidate refs; the metadata needed to choose lives on visible node {ref_0}.",
        ),
        t(
            "kernel_inspect",
            "bug_investigation",
            "after-trace",
            "happy",
            "Trace summarized the path, but the supersession metadata is only available on visible node {ref_1}.",
        ),
        t(
            "kernel_inspect",
            "smart_writing_session",
            "read-before-write",
            "happy",
            "A write between refs is prepared, but visible target {ref_0} must be verified before commit.",
        ),
        t(
            "kernel_inspect",
            "product_planning",
            "ambiguous-ref",
            "adversarial",
            "Tempting to inspect an ambiguous missing ref, but {ref_0} is the only visible candidate.",
        ),
        t(
            "kernel_inspect",
            "software_migration",
            "metadata",
            "happy",
            "Node {ref_1} is visible, but timestamps, source and agent metadata are not present in the summary.",
        ),
    ],
    "kernel_ingest": [
        t("kernel_ingest", "smart_writing_session", "rich-relation", "happy", "Execute the prepared kernel_ingest action exactly."),
        t("kernel_ingest", "technical_incident", "anemic-fallback", "adversarial", "Execute the prepared kernel_ingest fallback exactly."),
        t("kernel_ingest", "product_planning", "after-pre-read", "happy", "Execute the prepared canonical ingest after writer pre-read."),
        t("kernel_ingest", "bug_investigation", "missing-provenance", "adversarial", "Execute the visible prepared kernel_ingest action exactly; do not reconstruct missing fields."),
        t("kernel_ingest", "software_migration", "declared-dimensions", "happy", "Execute the prepared kernel_ingest payload with declared dimensions."),
    ],
    "kernel_write_memory": [
        t("kernel_write_memory", "smart_writing_session", "prepared-payload", "happy", "Execute the prepared kernel_write_memory action exactly."),
        t("kernel_write_memory", "technical_incident", "smart-proof", "happy", "Execute the prepared write with related evidence {ref_0}."),
        t("kernel_write_memory", "bug_investigation", "no-read-context", "adversarial", "Execute only the prepared kernel_write_memory action; do not invent extra refs."),
        t("kernel_write_memory", "product_planning", "related-refs", "happy", "Execute the prepared write with the visible related refs."),
        t("kernel_write_memory", "software_migration", "minimal", "happy", "Execute the minimal prepared kernel_write_memory action exactly."),
    ],
    "stop": [
        t(
            "stop",
            "technical_incident",
            "answer-ready",
            "happy",
            "Visible refs include symptom, cause and resolution; another read would not reduce uncertainty about the answer.",
        ),
        t(
            "stop",
            "bug_investigation",
            "no-candidate",
            "happy",
            "Tools have been exhausted on this about; remaining budget would not produce a ref that changes the answer.",
        ),
        t(
            "stop",
            "software_migration",
            "budget-exhausted",
            "happy",
            "Budget has dropped too low for another bounded call; the question cannot be resolved within limits.",
        ),
        t("stop", "product_planning", "premature-temptation", "adversarial", "Stop with answer_ready using only visible evidence {ref_0}; do not call another tool."),
        t(
            "stop",
            "smart_writing_session",
            "after-escalate-attempt",
            "adversarial",
            "Writer state has no executable candidate; a previous escalation attempt was rejected, leaving a bounded terminal answer.",
        ),
        t(
            "stop",
            "smart_writing_session",
            "premature-ask-temptation",
            "adversarial",
            "Tempting to ask about {ref_0}, but visible memory suggests no relevant answer and stopping is the honest bounded result.",
        ),
    ],
    "escalate": [
        t(
            "escalate",
            "technical_incident",
            "beyond-capability",
            "happy",
            "The next decision requires causal interpretation across contradictory evidence; bounded retrieval cannot resolve it.",
        ),
        t(
            "escalate",
            "bug_investigation",
            "after-reads",
            "happy",
            "Multiple reads completed; remaining ambiguity needs reasoning outside Operator's bounded tool surface.",
        ),
        t(
            "escalate",
            "product_planning",
            "ambiguous-scope",
            "adversarial",
            "About scope cannot be disambiguated from visible refs; safe bounded tools would not resolve the ambiguity.",
        ),
        t(
            "escalate",
            "software_migration",
            "do-not-speculate",
            "adversarial",
            "An invisible migration constraint is implied but cannot be retrieved from visible memory; speculating would invent fact.",
        ),
        t(
            "escalate",
            "smart_writing_session",
            "budget-alternative",
            "adversarial",
            "Write relation cannot be justified by current visible context, but write must occur eventually; escalation is bounded.",
        ),
        t(
            "escalate",
            "product_planning",
            "no-traceable-path",
            "adversarial",
            "Tempting to force a path between {ref_0} and {ref_1}, but visible refs do not prove one exists and escalation is the bounded path.",
        ),
    ],
}

EXTRA_TEMPLATES: list[Template] = [
    t(
        "kernel_wake",
        "smart_writing_session",
        "writer-pre-read-bootstrap",
        "happy",
        "A later write is planned for {about}, but target memory is not loaded. Refs cannot be validated until context exists.",
        mode="writer_pre_read",
    ),
    t(
        "kernel_ask",
        "smart_writing_session",
        "writer-pre-read-relation-class",
        "happy",
        "Prepared write carries a relation class hint; canonical memory context is needed before committing that relation.",
        mode="writer_pre_read",
    ),
    t(
        "kernel_near",
        "smart_writing_session",
        "writer-pre-read-target-candidates",
        "happy",
        "Prepared relation has visible candidates {ref_0} and {ref_1}; the correct anchor depends on the neighborhood in dimension {dim_0}.",
        mode="writer_pre_read",
    ),
    t(
        "kernel_inspect",
        "smart_writing_session",
        "writer-pre-read-proof",
        "happy",
        "Prepared write declares evidence ref {ref_0}; its metadata must be verified before any write is safe.",
        mode="writer_pre_read",
    ),
    t(
        "kernel_trace",
        "bug_investigation",
        "full-mode-causal-chain",
        "happy",
        "Read and write are both available, but causation between {ref_0} and {ref_1} must be reconstructed first.",
        mode="full",
    ),
    t(
        "stop",
        "product_planning",
        "full-mode-sufficient",
        "happy",
        "Write is available, but visible evidence already proves the answer; no further tool call would improve it.",
        mode="full",
    ),
]


def main() -> int:
    args = parse_args()
    scenarios = build_scenarios(args.count, args.seed)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    write_jsonl(args.output, scenarios)
    validate_with_cli(args.output)
    print_summary(args.output, scenarios)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--count", default=DEFAULT_SCENARIO_COUNT, type=positive_int)
    parser.add_argument("--seed", default=42, type=int)
    return parser.parse_args()


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("count must be positive")
    return parsed


def build_scenarios(count: int, seed: int) -> list[dict[str, Any]]:
    templates = interleaved_templates()
    scenarios = []
    for index in range(count):
        template = templates[index % len(templates)]
        variation = index // len(templates)
        rng = random.Random(f"{seed}:{template.target}:{template.slug}:{variation}")
        scenarios.append(render_scenario(template, index, variation, rng))
    return scenarios


def interleaved_templates() -> list[Template]:
    templates: list[Template] = []
    templates.extend(
        TEMPLATES_BY_TARGET[target][0] for target in TARGETS if TEMPLATES_BY_TARGET[target]
    )
    templates.extend(EXTRA_TEMPLATES)
    max_variants = max(len(target_templates) for target_templates in TEMPLATES_BY_TARGET.values())
    for variant_index in range(1, max_variants):
        for target in TARGETS:
            target_templates = TEMPLATES_BY_TARGET[target]
            if variant_index < len(target_templates):
                templates.append(target_templates[variant_index])
    if len(templates) != 66:
        raise RuntimeError(f"expected 66 templates, got {len(templates)}")
    return templates


def render_scenario(
    template: Template, index: int, variation: int, rng: random.Random
) -> dict[str, Any]:
    about = about_for(template, variation, rng)
    refs = refs_for(about, rng)
    dims = dims_for(rng)
    budget = budget_for(template, rng)
    cursor_anchor = f"seq:{variation + 1}"
    context = {
        "about": about,
        "ref_0": refs[0],
        "ref_1": refs[1],
        "ref_2": refs[2],
        "dim_0": dims[0],
        "dim_1": dims[1],
        "cursor_anchor": cursor_anchor,
    }
    mode = mode_for(template)
    subject: dict[str, Any] = {
        "about": about,
        "mode": mode,
        "task_family": f"realistic.{template.target}.{template.slug}",
        "goal": template.goal.format(**context),
        "allowed_tools": allowed_tools_for_mode(mode),
        "visible_state": visible_state(template.target, refs, dims, budget, cursor_anchor),
    }
    prepared = prepared_action_for(template, about, refs, dims, variation)
    if prepared is not None:
        subject["prepared_action"] = prepared
    return {
        "scenario_id": f"scenario:{template.target}:{template.slug}:{index:04}",
        "target": template.target,
        "subject": subject,
        "metadata": {
            "theme": template.theme,
            "category": template.category,
            "template": template.slug,
            "variation": variation,
        },
    }


def mode_for(template: Template) -> str:
    if template.mode is not None:
        return template.mode
    return "write" if template.target in {"kernel_ingest", "kernel_write_memory"} else "read"


def allowed_tools_for_mode(mode: str) -> list[str]:
    if mode == "write":
        return WRITE_TOOLS
    if mode == "writer_pre_read":
        return WRITER_PRE_READ_TOOLS
    if mode == "full":
        return FULL_TOOLS
    return READ_TOOLS


def refs_for(about: str, rng: random.Random) -> list[str]:
    tokens = rng.sample(REF_VOCAB, 4)
    return [f"{about}:node:{token}:{index:03}" for index, token in enumerate(tokens)]


def dims_for(rng: random.Random) -> list[str]:
    return rng.sample(DIM_VOCAB, 2)


def budget_for(template: Template, rng: random.Random) -> tuple[int, int]:
    if template.target == "stop" and "budget" in template.slug:
        return (0, 300)
    if template.category == "adversarial":
        return rng.choice([(1, 600), (2, 1200)])
    return rng.choice(BUDGET_VARIANTS)


def about_for(template: Template, variation: int, rng: random.Random) -> str:
    prefix = rng.choice(ABOUT_PREFIXES_BY_THEME[template.theme])
    case_number = f"case-{variation:03}"
    return f"about:{prefix}:{template.target}:{template.slug}:{case_number}"


def visible_state(
    target: str,
    refs: list[str],
    dims: list[str],
    budget: tuple[int, int],
    cursor_anchor: str,
) -> dict[str, Any]:
    state: dict[str, Any] = {
        "known_refs": refs,
        "known_dimensions": dims,
        "budget": {"calls_remaining": budget[0], "tokens_remaining": budget[1]},
    }
    if target in {"kernel_rewind", "kernel_forward"}:
        state["active_cursor"] = {
            "kind": "temporal",
            "key": "created",
            "anchor": cursor_anchor,
        }
    return state


def prepared_action_for(
    template: Template,
    about: str,
    refs: list[str],
    dims: list[str],
    variation: int,
) -> dict[str, Any] | None:
    if template.target == "kernel_write_memory":
        return tool_call(
            "kernel_write_memory",
            {
                "summary": f"Record {template.slug} decision.",
                "body": f"The prepared write links visible evidence {refs[0]} to the current process state.",
                "related": [refs[0], refs[1]],
            },
        )
    if template.target != "kernel_ingest":
        return None
    new_entry = f"{about}:entry:prepared:{template.slug}:{variation:03}"
    relation = {
        "from": refs[0],
        "to": new_entry,
        "rel": "chosen_because" if "anemic" not in template.slug else "follows",
        "class": "causal" if "anemic" not in template.slug else "procedural",
        "why": f"The prepared ingest uses visible evidence {refs[0]}.",
        "evidence": "visible evidence supports the prepared entry",
        "confidence": "high" if "anemic" not in template.slug else "medium",
        "sequence": variation + 1,
    }
    return tool_call(
        "kernel_ingest",
        {
            "about": about,
            "memory": {
                "dimensions": [
                    {
                        "id": dims[0],
                        "kind": "agent",
                        "title": "Operator writer",
                        "metadata": {},
                    }
                ],
                "entries": [
                    {
                        "id": new_entry,
                        "kind": "decision",
                        "text": f"Prepared ingest for {template.slug}.",
                        "coordinates": [
                            {
                                "dimension": dims[0],
                                "scope_id": about,
                                "sequence": variation + 1,
                                "metadata": {},
                            }
                        ],
                        "metadata": {"template": template.slug},
                    }
                ],
                "relations": [relation],
                "evidence": [
                    {
                        "id": f"evidence:{template.slug}:{variation:03}",
                        "supports": [refs[0], new_entry],
                        "text": f"Visible evidence {refs[0]} supports {new_entry}.",
                        "metadata": {},
                    }
                ],
            },
            "provenance": {
                "source_kind": "agent",
                "source_agent": "operator-scenario-builder",
                "observed_at": "2026-05-22T00:00:00Z",
                "correlation_id": f"corr:{template.slug}:{variation:03}",
                "causation_id": f"cause:{template.slug}:{variation:03}",
            },
            "idempotency_key": f"idem:{template.slug}:{variation:03}",
            "dry_run": True,
        },
    )


def tool_call(tool: str, arguments: dict[str, Any]) -> dict[str, Any]:
    return {"kind": "tool_call", "tool": tool, "arguments": arguments}


def write_jsonl(output: Path, rows: list[dict[str, Any]]) -> None:
    with output.open("w", encoding="utf-8") as writer:
        for row in rows:
            # ScenarioDto ignores metadata today; it is kept for human audit of
            # external artifacts and does not cross the Rust domain boundary.
            writer.write(json.dumps(row, ensure_ascii=False, sort_keys=True, separators=(",", ":")))
            writer.write("\n")


def validate_with_cli(output: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="operator-scenario-validate-") as tmp:
        tmp_path = Path(tmp)
        key = tmp_path / "openai.txt"
        prompt = tmp_path / "prompt.md"
        out = tmp_path / "out"
        key.write_text("validate-only-key\n", encoding="utf-8")
        prompt.write_text("validate scenarios only\n", encoding="utf-8")
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "-p",
                "operator-synthetic-cli",
                "--bin",
                "operator-realistic-corpus",
                "--",
                "--scenarios",
                str(output),
                "--output",
                str(out),
                "--api-base",
                "https://api.openai.com/v1",
                "--api-key-file",
                str(key),
                "--prompt",
                str(prompt),
                "--model",
                "validate-only",
                "--validate-only",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise SystemExit(
                "generated scenarios.jsonl does not parse\n"
                f"stdout:\n{result.stdout}\n"
                f"stderr:\n{result.stderr}"
            )


def print_summary(output: Path, scenarios: list[dict[str, Any]]) -> None:
    by_target: dict[str, int] = {}
    by_category: dict[str, int] = {}
    for row in scenarios:
        by_target[row["target"]] = by_target.get(row["target"], 0) + 1
        category = row["metadata"]["category"]
        by_category[category] = by_category.get(category, 0) + 1
    print(f"wrote {len(scenarios)} scenarios to {output}")
    print("by_target:", json.dumps(by_target, sort_keys=True))
    print("by_category:", json.dumps(by_category, sort_keys=True))


if __name__ == "__main__":
    raise SystemExit(main())
