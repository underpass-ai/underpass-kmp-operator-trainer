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
