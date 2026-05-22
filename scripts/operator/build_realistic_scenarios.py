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


def t(target: str, theme: str, slug: str, category: str, goal: str) -> Template:
    return Template(target=target, theme=theme, slug=slug, category=category, goal=goal)


TEMPLATES_BY_TARGET: dict[str, list[Template]] = {
    "kernel_wake": [
        t("kernel_wake", "technical_incident", "current-about", "happy", "Call kernel_wake for {about} before deeper navigation."),
        t("kernel_wake", "software_migration", "prior-context", "happy", "Open {about} with kernel_wake to recover prior migration context."),
        t("kernel_wake", "smart_writing_session", "before-write", "happy", "Use kernel_wake on {about} before any write decision."),
        t("kernel_wake", "product_planning", "after-stop-review", "adversarial", "Despite the low budget, call kernel_wake for {about}; no refs are visible yet."),
        t("kernel_wake", "bug_investigation", "already-loaded-check", "adversarial", "Call kernel_wake for {about} to establish the bounded investigation scope."),
    ],
    "kernel_ask": [
        t("kernel_ask", "technical_incident", "evidence-query", "happy", "Call kernel_ask with a narrow query for evidence about {ref_0}."),
        t("kernel_ask", "product_planning", "clarify-choice", "happy", "Use kernel_ask to retrieve the deterministic planning fact for {about}."),
        t("kernel_ask", "bug_investigation", "narrow-fact", "happy", "Call kernel_ask for the exact fact that links {ref_0} to {ref_1}."),
        t("kernel_ask", "software_migration", "missing-constraint", "happy", "Use kernel_ask to find the migration constraint before moving in the graph."),
        t("kernel_ask", "smart_writing_session", "no-relevant-memory", "adversarial", "Call kernel_ask with a bounded query because visible refs do not answer the write question."),
    ],
    "kernel_near": [
        t("kernel_near", "technical_incident", "relations", "happy", "Call kernel_near around {ref_0} in dimension {dim_0} with limit 4."),
        t("kernel_near", "bug_investigation", "candidate-refs", "happy", "Use kernel_near to expand around candidate {ref_0} before selecting evidence."),
        t("kernel_near", "product_planning", "ambiguous-anchor", "adversarial", "Call kernel_near around the visible anchor {ref_0}; do not invent another anchor."),
        t("kernel_near", "software_migration", "limit-pressure", "happy", "Call kernel_near around {ref_0} with a small limit because budget is bounded."),
        t("kernel_near", "smart_writing_session", "dimension-filter", "happy", "Use kernel_near around {ref_0} filtered to dimension {dim_0}."),
    ],
    "kernel_goto": [
        t("kernel_goto", "technical_incident", "ref", "happy", "Call kernel_goto to jump to visible ref {ref_0}."),
        t("kernel_goto", "software_migration", "temporal-cursor", "happy", "Call kernel_goto with a ref cursor targeting {ref_0} before moving in time."),
        t("kernel_goto", "bug_investigation", "trace-cursor", "happy", "Call kernel_goto to jump to the trace endpoint {ref_1}."),
        t("kernel_goto", "product_planning", "invented-ref-temptation", "adversarial", "Call kernel_goto only to the visible ref {ref_0}; do not invent a planning ref."),
        t("kernel_goto", "smart_writing_session", "cross-about", "adversarial", "Call kernel_goto to the visible same-about ref {ref_0}; avoid cross-about movement."),
    ],
    "kernel_rewind": [
        t("kernel_rewind", "technical_incident", "prior-state", "happy", "Call kernel_rewind on the active created cursor with window 2."),
        t("kernel_rewind", "bug_investigation", "verification", "happy", "Use kernel_rewind to inspect the prior state before {ref_0}."),
        t("kernel_rewind", "software_migration", "missing-anchor", "adversarial", "Call kernel_rewind using the active temporal cursor; do not create a new anchor."),
        t("kernel_rewind", "product_planning", "refute", "happy", "Use kernel_rewind to check what was known before {ref_1}."),
        t("kernel_rewind", "smart_writing_session", "multi-step", "happy", "Call kernel_rewind with window 2 to recover read-before-write context."),
    ],
    "kernel_forward": [
        t("kernel_forward", "technical_incident", "current-state", "happy", "Call kernel_forward on the active created cursor with window 2."),
        t("kernel_forward", "software_migration", "after-rewind", "happy", "Use kernel_forward after rewind to see what changed after {ref_0}."),
        t("kernel_forward", "bug_investigation", "adversarial-anchor", "adversarial", "Call kernel_forward using the visible active cursor; do not alter the anchor."),
        t("kernel_forward", "product_planning", "next-event", "happy", "Use kernel_forward to move to the next planning event."),
        t("kernel_forward", "smart_writing_session", "across-sessions", "happy", "Call kernel_forward to continue the current session timeline."),
    ],
    "kernel_trace": [
        t("kernel_trace", "technical_incident", "causal", "happy", "Call kernel_trace from {ref_0} to {ref_1} with page 8."),
        t("kernel_trace", "bug_investigation", "contradiction", "happy", "Use kernel_trace to explain how {ref_1} contradicted {ref_0}."),
        t("kernel_trace", "software_migration", "supersession", "happy", "Call kernel_trace from stale plan {ref_0} to final plan {ref_1}."),
        t("kernel_trace", "product_planning", "no-path", "adversarial", "Call kernel_trace only between the visible refs {ref_0} and {ref_1}."),
        t("kernel_trace", "smart_writing_session", "continued-page", "happy", "Use kernel_trace from {ref_0} to {ref_1} with an explicit first page."),
    ],
    "kernel_inspect": [
        t("kernel_inspect", "technical_incident", "after-near", "happy", "Call kernel_inspect on visible ref {ref_0}."),
        t("kernel_inspect", "bug_investigation", "after-trace", "happy", "Inspect {ref_1} because the trace summary is not enough."),
        t("kernel_inspect", "smart_writing_session", "read-before-write", "happy", "Use kernel_inspect on {ref_0} to prove read-before-write context."),
        t("kernel_inspect", "product_planning", "ambiguous-ref", "adversarial", "Inspect the visible ref {ref_0}; do not choose the ambiguous missing ref."),
        t("kernel_inspect", "software_migration", "metadata", "happy", "Call kernel_inspect on {ref_1} to read rich metadata."),
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
        t("stop", "technical_incident", "answer-ready", "happy", "Stop with answer_ready because {ref_0} already proves the answer."),
        t("stop", "bug_investigation", "no-candidate", "happy", "Stop because there is no visible candidate worth another tool call."),
        t("stop", "software_migration", "budget-exhausted", "happy", "Stop because the remaining budget is exhausted."),
        t("stop", "product_planning", "premature-temptation", "adversarial", "Stop with answer_ready using only visible evidence {ref_0}; do not call another tool."),
        t("stop", "smart_writing_session", "after-escalate-attempt", "adversarial", "Stop because the prepared writer state has no executable candidate."),
    ],
    "escalate": [
        t("escalate", "technical_incident", "beyond-capability", "happy", "Escalate with beyond_capability because causal interpretation is outside Operator."),
        t("escalate", "bug_investigation", "after-reads", "happy", "Escalate after multiple reads because the next decision needs a larger reasoner."),
        t("escalate", "product_planning", "ambiguous-scope", "adversarial", "Escalate because the about scope is ambiguous and no safe tool call resolves it."),
        t("escalate", "software_migration", "do-not-speculate", "adversarial", "Escalate rather than speculating about an invisible migration constraint."),
        t("escalate", "smart_writing_session", "budget-alternative", "adversarial", "Escalate because the write relation cannot be justified by visible context."),
    ],
}


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
    parser.add_argument("--count", default=1500, type=positive_int)
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
    for variant_index in range(5):
        for target in TARGETS:
            templates.append(TEMPLATES_BY_TARGET[target][variant_index])
    if len(templates) != 60:
        raise RuntimeError(f"expected 60 templates, got {len(templates)}")
    return templates


def render_scenario(
    template: Template, index: int, variation: int, rng: random.Random
) -> dict[str, Any]:
    refs = refs_for(template, variation, rng)
    dims = dims_for(rng)
    budget = budget_for(template, rng)
    about = about_for(template, variation, rng)
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
    mode = mode_for(template.target)
    subject: dict[str, Any] = {
        "about": about,
        "mode": mode,
        "task_family": f"realistic.{template.target}.{template.slug}",
        "goal": template.goal.format(**context),
        "allowed_tools": WRITE_TOOLS if mode == "write" else READ_TOOLS,
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


def mode_for(target: str) -> str:
    return "write" if target in {"kernel_ingest", "kernel_write_memory"} else "read"


def refs_for(template: Template, variation: int, rng: random.Random) -> list[str]:
    tokens = rng.sample(REF_VOCAB, 4)
    return [
        f"node:{template.theme}:{template.slug}:{token}:{variation:03}"
        for token in tokens
    ]


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
    # Reuse about ids across nearby variations so the final corpus contains
    # multi-step/multi-scenario processes instead of isolated rows only.
    return f"{prefix}:case-{variation // 4:03}"


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
