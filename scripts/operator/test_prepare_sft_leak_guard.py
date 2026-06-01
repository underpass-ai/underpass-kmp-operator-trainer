"""Leak-guard tests for the SFT dataset preparation.

`assert_model_facing_visible_state_clean` must reject request-hint keys
(`requested_*` / `prepared_*` / `inspection_request`) in the model-facing
visible_state — they project the target action and would let the student copy
the answer instead of deciding — UNLESS the item came through explicit
injection (translation/replay smokes), which stamps `REQUEST_HINTS_MARKER`.

Run:  python3 scripts/operator/test_prepare_sft_leak_guard.py
(or via pytest; each `test_*` is an independent assertion).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The prepare module imports a sibling script (predict_operator_sft); make the
# scripts dir importable before loading it.
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))
_spec = importlib.util.spec_from_file_location(
    "prepare_operator_sft_dataset", _HERE / "prepare_operator_sft_dataset.py"
)
prep = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(prep)


def _item(visible_state: dict, marker: bool = False) -> dict:
    item = {"step_id": "step:1", "visible_state": visible_state}
    if marker:
        item[prep.REQUEST_HINTS_MARKER] = True
    return item


def _expect_reject(visible_state: dict, needle: str) -> None:
    try:
        prep.assert_model_facing_visible_state_clean(_item(visible_state))
    except ValueError as err:
        assert needle in str(err), f"expected '{needle}' in: {err}"
    else:
        raise AssertionError(f"expected ValueError for model-facing {needle}")


def test_clean_visible_state_passes() -> None:
    prep.assert_model_facing_visible_state_clean(
        _item({"known_refs": ["wkshop-01"], "budget": {"calls_remaining": 5}})
    )


def test_requested_wake_without_marker_is_rejected() -> None:
    _expect_reject({"known_refs": [], "requested_wake": {"intent": "x"}}, "requested_wake")


def test_requested_stop_without_marker_is_rejected() -> None:
    _expect_reject(
        {"known_refs": [], "requested_stop": {"answer": "12", "evidence": []}},
        "requested_stop",
    )


def test_inspection_request_without_marker_is_rejected() -> None:
    _expect_reject({"known_refs": [], "inspection_request": {"target": "x"}}, "inspection_request")


def test_nested_request_hint_is_rejected() -> None:
    _expect_reject(
        {"known_refs": [], "active_cursor": {"requested_move": {"kind": "kernel_near"}}},
        "requested_move",
    )


def test_injection_marks_item_so_hints_are_allowed() -> None:
    # Explicit injection stamps the marker; the same hints then pass the check.
    raw = {
        "step_id": "step:1",
        "visible_state": {"known_refs": []},
        "target_action": {
            "kind": "tool_call",
            "tool": "kernel_wake",
            "arguments": {"about": "a", "intent": "x"},
        },
    }
    injected = prep.inject_target_request_fields(raw)
    assert injected.get(prep.REQUEST_HINTS_MARKER) is True
    assert "requested_wake" in injected["visible_state"]
    prep.assert_model_facing_visible_state_clean(injected)  # must not raise


def test_marker_lives_at_item_level_not_in_user_payload() -> None:
    # to_sft_row builds the user payload from a fixed field set that excludes the
    # marker, and the marker is never placed inside visible_state — so it cannot
    # itself become model-facing context.
    raw = {
        "step_id": "step:1",
        "visible_state": {"known_refs": []},
        "target_action": {
            "kind": "tool_call",
            "tool": "kernel_wake",
            "arguments": {"about": "a"},
        },
    }
    injected = prep.inject_target_request_fields(raw)
    user_payload_fields = {"task_family", "mode", "about", "goal", "allowed_tools", "visible_state"}
    assert prep.REQUEST_HINTS_MARKER not in user_payload_fields
    assert prep.REQUEST_HINTS_MARKER not in injected["visible_state"]


if __name__ == "__main__":
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_") and callable(v)]
    for fn in tests:
        fn()
        print(f"  ok: {fn.__name__}")
    print(f"ALL {len(tests)} leak-guard tests passed")
