#!/usr/bin/env python3
"""Validate OpenAI-format SFT JSONL emitted by prepare_operator_sft_dataset.py."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


REQUIRED_TOP_LEVEL = {"step_id", "messages"}
EXPECTED_ROLES = ["system", "user", "assistant"]


def main() -> int:
    if len(sys.argv) < 2:
        raise SystemExit("usage: validate_openai_sft_schema.py OPENAI_JSONL [...]")
    for raw_path in sys.argv[1:]:
        validate_path(Path(raw_path))
    print(f"OK: schema validated for {sys.argv[1:]}")
    return 0


def validate_path(path: Path) -> None:
    if not path.is_file():
        raise SystemExit(f"{path}: file does not exist")
    rows = 0
    with path.open(encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            if not line.strip():
                continue
            rows += 1
            row = parse_row(path, line_no, line)
            validate_row(path, line_no, row)
    if rows == 0:
        raise SystemExit(f"{path}: file is empty")


def parse_row(path: Path, line_no: int, line: str) -> dict[str, Any]:
    try:
        row = json.loads(line)
    except json.JSONDecodeError as err:
        raise SystemExit(f"{path}:{line_no}: invalid JSON: {err}") from err
    if not isinstance(row, dict):
        raise SystemExit(f"{path}:{line_no}: row must be an object")
    return row


def validate_row(path: Path, line_no: int, row: dict[str, Any]) -> None:
    missing = REQUIRED_TOP_LEVEL - set(row.keys())
    if missing:
        raise SystemExit(f"{path}:{line_no}: missing top-level fields {sorted(missing)}")
    step_id = row["step_id"]
    if not isinstance(step_id, str) or not step_id:
        raise SystemExit(f"{path}:{line_no}: step_id must be non-empty string")
    messages = row["messages"]
    if not isinstance(messages, list) or len(messages) != 3:
        raise SystemExit(f"{path}:{line_no}: messages must be a list of 3 entries")
    roles = []
    for index, message in enumerate(messages):
        if not isinstance(message, dict):
            raise SystemExit(f"{path}:{line_no}: message {index} must be an object")
        role = message.get("role")
        content = message.get("content")
        roles.append(role)
        if not isinstance(content, str) or not content:
            raise SystemExit(
                f"{path}:{line_no}: message {index} content must be non-empty string"
            )
    if roles != EXPECTED_ROLES:
        raise SystemExit(
            f"{path}:{line_no}: expected roles {EXPECTED_ROLES}, got {roles}"
        )


if __name__ == "__main__":
    raise SystemExit(main())
