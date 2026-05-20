#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
TMP_DIR="${TMPDIR:-/tmp}/operator-round-trip-smoke-$$"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

mkdir -p "${TMP_DIR}"

TRAJECTORIES="${TMP_DIR}/trajectories.jsonl"
SFT_DIR="${TMP_DIR}/sft"
PREDICTIONS_DIR="${TMP_DIR}/predictions"
PREDICTIONS="${PREDICTIONS_DIR}/predictions.jsonl"

cargo run --quiet -p operator-synthetic-cli --bin operator-synthesize -- \
  --dataset-id "dataset:round-trip-smoke" \
  --minimum-examples 1 \
  --output "${TRAJECTORIES}"

cargo run --quiet -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories "${TRAJECTORIES}" \
  --require-full-coverage \
  --require-zero-invalid

python3 "${ROOT}/scripts/operator/prepare_operator_sft_dataset.py" \
  --trajectories "${TRAJECTORIES}" \
  --output "${SFT_DIR}" \
  --limit 5 \
  --eval-ratio 0.4 \
  --force

python3 "${ROOT}/scripts/operator/audit_operator_sft_no_gold.py" \
  "${SFT_DIR}/openai_train.jsonl" \
  "${SFT_DIR}/openai_eval.jsonl" \
  --output "${TMP_DIR}/no_gold_audit.json"

python3 "${ROOT}/scripts/operator/train_operator_sft_lora.py" \
  --train-jsonl "${SFT_DIR}/openai_train.jsonl" \
  --eval-jsonl "${SFT_DIR}/openai_eval.jsonl" \
  --output-dir "${TMP_DIR}/train-validate-only" \
  --validate-only

python3 "${ROOT}/scripts/operator/predict_operator_sft.py" \
  --dataset-jsonl "${SFT_DIR}/eval.jsonl" \
  --model-id "smoke-validation-only" \
  --output "${TMP_DIR}/predict-validate-only" \
  --resolve-prepared-payloads \
  --validate-only

mkdir -p "${PREDICTIONS_DIR}"
python3 - "${SFT_DIR}/eval_trajectories.jsonl" "${PREDICTIONS}" <<'PY'
import json
import sys
from pathlib import Path

source = Path(sys.argv[1])
target = Path(sys.argv[2])
with source.open(encoding="utf-8") as reader, target.open("w", encoding="utf-8") as writer:
    for line in reader:
        if not line.strip():
            continue
        row = json.loads(line)
        writer.write(
            json.dumps(
                {"step_id": row["step_id"], "action": row["target_action"]},
                separators=(",", ":"),
                sort_keys=True,
            )
            + "\n"
        )
PY

cargo run --quiet -p operator-evaluation-cli --bin operator-policy-eval -- \
  --predictions "${PREDICTIONS}" \
  --ground-truth "${SFT_DIR}/eval_trajectories.jsonl" \
  --min-pass-rate 1.0

printf 'operator round-trip smoke ok: %s\n' "${TMP_DIR}"
