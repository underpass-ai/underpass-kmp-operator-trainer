#!/usr/bin/env python3
"""Verify semantic acceptance rules for realistic-v7 scenario artifacts."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


TARGETS = {
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
}

WRITE_TARGETS = {"kernel_ingest", "kernel_write_memory"}

FORBIDDEN_HAPPY_PATTERNS = [
    re.compile(r"\bcall kernel_\w+"),
    re.compile(r"\buse kernel_\w+"),
    re.compile(r"\bexecute kernel_\w+"),
    re.compile(r"\bstop with\b"),
    re.compile(r"\bescalate with\b"),
    re.compile(r"\bwith page \d+"),
    re.compile(r"\bwith limit \d+"),
    re.compile(r"\bwith window \d+"),
]


def main() -> int:
    args = parse_args()
    cases = read_jsonl(args.scenarios)
    verify_total_count(cases)
    verify_unique_abouts(cases)
    verify_modes(cases)
    verify_target_coverage(cases)
    verify_refs_share_about_prefix(cases)
    verify_adversarial_goal_target_consistency(cases)
    verify_happy_goals_are_situational(cases)
    verify_near_goals_expose_anchor_and_dimension(cases)
    verify_theme_balance(cases)
    print_summary(cases)
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenarios", type=Path)
    return parser.parse_args()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    cases = []
    with path.open("r", encoding="utf-8") as reader:
        for line_number, line in enumerate(reader, start=1):
            if not line.strip():
                continue
            try:
                cases.append(json.loads(line))
            except json.JSONDecodeError as err:
                raise AssertionError(f"line {line_number}: invalid JSON: {err}") from err
    return cases


def verify_total_count(cases: list[dict[str, Any]]) -> None:
    assert len(cases) >= 1500, f"need at least 1500 scenarios, got {len(cases)}"


def verify_unique_abouts(cases: list[dict[str, Any]]) -> None:
    abouts = [case["subject"]["about"] for case in cases]
    assert len(set(abouts)) == len(abouts), "about IDs must be unique"


def verify_modes(cases: list[dict[str, Any]]) -> None:
    modes = count_by(cases, lambda case: case["subject"]["mode"])
    writer_pre_read = modes.get("writer_pre_read", 0)
    full = modes.get("full", 0)
    assert writer_pre_read >= 100, (
        f"need at least 100 writer_pre_read scenarios, got {writer_pre_read}"
    )
    assert full >= 50, f"need at least 50 full-mode scenarios, got {full}"


def verify_target_coverage(cases: list[dict[str, Any]]) -> None:
    found = {case["target"] for case in cases}
    missing = sorted(TARGETS - found)
    assert not missing, f"missing targets: {missing}"


def verify_refs_share_about_prefix(cases: list[dict[str, Any]]) -> None:
    violations = []
    for case in cases:
        about = case["subject"]["about"]
        node_prefix = f"{about}:node:"
        entry_prefix = f"{about}:entry:"
        for field, ref, allow_entry in iter_ref_surfaces(case):
            if not isinstance(ref, str):
                violations.append(
                    f"{case['scenario_id']}: {field} ref must be string, got {type(ref).__name__}"
                )
                continue
            if not ref.startswith("about:"):
                violations.append(
                    f"{case['scenario_id']}: {field} ref {ref!r} does not start with 'about:'"
                )
                continue
            if ref.startswith(node_prefix):
                continue
            if allow_entry and ref.startswith(entry_prefix):
                continue
            expected = f"{node_prefix}*"
            if allow_entry:
                expected = f"{node_prefix}* or {entry_prefix}*"
            violations.append(
                f"{case['scenario_id']}: {field} ref {ref!r} does not match {expected}"
            )
    assert not violations, "ref scope violations; first 10: " + repr(violations[:10])


def iter_ref_surfaces(case: dict[str, Any]) -> list[tuple[str, str, bool]]:
    subject = case["subject"]
    state = subject["visible_state"]
    refs: list[tuple[str, str, bool]] = []
    for ref in state.get("known_refs", []):
        refs.append(("visible_state.known_refs", ref, False))
    current_ref = state.get("current_ref")
    if current_ref:
        refs.append(("visible_state.current_ref", current_ref, False))
    for ref in state.get("last_observed_refs", []) or []:
        refs.append(("visible_state.last_observed_refs", ref, False))
    cursor = state.get("active_cursor")
    if isinstance(cursor, dict):
        if cursor.get("kind") == "ref" and cursor.get("target"):
            refs.append(("visible_state.active_cursor.target", cursor["target"], False))
        if cursor.get("kind") == "trace":
            if cursor.get("from"):
                refs.append(("visible_state.active_cursor.from", cursor["from"], False))
            if cursor.get("to"):
                refs.append(("visible_state.active_cursor.to", cursor["to"], False))
    prepared = subject.get("prepared_action")
    if isinstance(prepared, dict):
        refs.extend(iter_refs_in_action("subject.prepared_action", prepared))
    return refs


def iter_refs_in_action(prefix: str, action: dict[str, Any]) -> list[tuple[str, str, bool]]:
    if action.get("kind") != "tool_call":
        return []
    tool = action.get("tool")
    args = action.get("arguments", {})
    refs: list[tuple[str, str, bool]] = []
    if not isinstance(args, dict):
        return refs
    if tool == "kernel_inspect" and args.get("target"):
        refs.append((f"{prefix}.arguments.target", args["target"], False))
    elif tool == "kernel_near" and args.get("anchor"):
        refs.append((f"{prefix}.arguments.anchor", args["anchor"], False))
    elif tool == "kernel_goto":
        refs.extend(iter_refs_in_cursor(f"{prefix}.arguments.cursor", args.get("cursor")))
    elif tool == "kernel_trace":
        if args.get("from"):
            refs.append((f"{prefix}.arguments.from", args["from"], False))
        if args.get("to"):
            refs.append((f"{prefix}.arguments.to", args["to"], False))
    elif tool == "kernel_write_memory":
        for ref in args.get("related", []) or []:
            refs.append((f"{prefix}.arguments.related", ref, False))
    elif tool == "kernel_ingest":
        memory = args.get("memory", {})
        if isinstance(memory, dict):
            for entry in memory.get("entries", []) or []:
                if not isinstance(entry, dict):
                    continue
                if entry.get("id"):
                    refs.append((f"{prefix}.arguments.memory.entries.id", entry["id"], True))
            for relation in memory.get("relations", []) or []:
                if not isinstance(relation, dict):
                    continue
                if relation.get("from"):
                    refs.append(
                        (f"{prefix}.arguments.memory.relations.from", relation["from"], True)
                    )
                if relation.get("to"):
                    refs.append(
                        (f"{prefix}.arguments.memory.relations.to", relation["to"], True)
                    )
            for evidence in memory.get("evidence", []) or []:
                if not isinstance(evidence, dict):
                    continue
                for ref in evidence.get("supports", []) or []:
                    refs.append((f"{prefix}.arguments.memory.evidence.supports", ref, True))
    return refs


def iter_refs_in_cursor(prefix: str, cursor: Any) -> list[tuple[str, str, bool]]:
    if not isinstance(cursor, dict):
        return []
    if cursor.get("kind") == "ref" and cursor.get("target"):
        return [(f"{prefix}.target", cursor["target"], False)]
    if cursor.get("kind") == "trace":
        refs = []
        if cursor.get("from"):
            refs.append((f"{prefix}.from", cursor["from"], False))
        if cursor.get("to"):
            refs.append((f"{prefix}.to", cursor["to"], False))
        return refs
    return []


ADVERSARIAL_PATTERNS_TO_REQUIRED_TARGET = [
    (re.compile(r"\bmay require stopping\b"), "stop"),
    (re.compile(r"\bstopping is the honest\b"), "stop"),
    (re.compile(r"\bbounded answer is to stop\b"), "stop"),
    (re.compile(r"\bmay require escalation\b"), "escalate"),
    (re.compile(r"\bneeds a larger reasoner\b"), "escalate"),
    (re.compile(r"\bescalation is the bounded path\b"), "escalate"),
    (re.compile(r"\bhonesty requires\b"), None),
]


def verify_adversarial_goal_target_consistency(cases: list[dict[str, Any]]) -> None:
    violations = []
    for case in cases:
        if case["metadata"]["category"] != "adversarial":
            continue
        goal = case["subject"]["goal"].lower()
        target = case["target"]
        for pattern, required in ADVERSARIAL_PATTERNS_TO_REQUIRED_TARGET:
            if not pattern.search(goal):
                continue
            if required is None:
                violations.append(
                    f"{case['scenario_id']}: ambiguous adversarial phrase {pattern.pattern!r}"
                )
            elif target != required:
                violations.append(
                    f"{case['scenario_id']}: goal suggests {required!r} but target is {target!r}"
                )
    assert not violations, (
        "adversarial goal/target contradictions; "
        f"first 10 violations: {violations[:10]}"
    )


def verify_happy_goals_are_situational(cases: list[dict[str, Any]]) -> None:
    violations = []
    for case in cases:
        if case["metadata"]["category"] != "happy":
            continue
        if case["target"] in WRITE_TARGETS:
            continue
        goal = case["subject"]["goal"].lower()
        for pattern in FORBIDDEN_HAPPY_PATTERNS:
            if pattern.search(goal):
                violations.append((case["scenario_id"], pattern.pattern, goal))
                break
    assert not violations, (
        "happy goals contain instruction patterns; "
        f"first 5 violations: {violations[:5]}"
    )


def verify_near_goals_expose_anchor_and_dimension(cases: list[dict[str, Any]]) -> None:
    violations = []
    for case in cases:
        if case["target"] != "kernel_near":
            continue
        goal = case["subject"]["goal"]
        refs = case["subject"]["visible_state"]["known_refs"]
        dimensions = case["subject"]["visible_state"]["known_dimensions"]
        has_ref = any(ref in goal for ref in refs)
        has_dimension = any(dimension in goal for dimension in dimensions)
        if not has_ref or not has_dimension:
            violations.append(
                {
                    "scenario_id": case["scenario_id"],
                    "has_ref": has_ref,
                    "has_dimension": has_dimension,
                    "goal": goal,
                }
            )
    assert not violations, (
        "kernel_near goals must expose a visible anchor and dimension; "
        f"first 5 violations: {violations[:5]}"
    )


def verify_theme_balance(cases: list[dict[str, Any]]) -> None:
    themes = {case["metadata"]["theme"] for case in cases}
    assert len(themes) == 5, f"expected 5 themes, got {sorted(themes)}"


def count_by(cases: list[dict[str, Any]], key_fn: Any) -> dict[str, int]:
    counts: dict[str, int] = {}
    for case in cases:
        key = key_fn(case)
        counts[key] = counts.get(key, 0) + 1
    return counts


def print_summary(cases: list[dict[str, Any]]) -> None:
    print(f"verified {len(cases)} scenarios")
    print("by_target:", json.dumps(count_by(cases, lambda case: case["target"]), sort_keys=True))
    print(
        "by_mode:",
        json.dumps(count_by(cases, lambda case: case["subject"]["mode"]), sort_keys=True),
    )
    print(
        "by_category:",
        json.dumps(count_by(cases, lambda case: case["metadata"]["category"]), sort_keys=True),
    )


if __name__ == "__main__":
    raise SystemExit(main())
