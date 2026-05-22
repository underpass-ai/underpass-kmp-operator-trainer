#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RUN_ID="${OPERATOR_RUN_ID:-realistic-v7-$(date -u +%Y%m%dT%H%M%SZ)}"
ART="${OPERATOR_ARTIFACT_ROOT:-${ROOT}/../rehydration-kernel-artifacts/operator}"
OUT="${ART}/${RUN_ID}"
SCENARIOS="${OPERATOR_SCENARIOS:-${ART}/scenarios-v1/scenarios.jsonl}"
LIMIT="${OPERATOR_RUN_LIMIT:-0}"
MODEL="${OPERATOR_MODEL:-gpt-4o-mini}"
API_BASE="${OPERATOR_API_BASE:-https://api.openai.com/v1}"
API_KEY_FILE="${OPERATOR_API_KEY_FILE:-/tmp/openai.txt}"
if [[ -z "${OPERATOR_PROMPT:-}" ]]; then
  echo "OPERATOR_PROMPT is required, for example: crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md" >&2
  exit 2
fi
PROMPT="${OPERATOR_PROMPT}"

mkdir -p "${OUT}"
cd "${ROOT}"

LIMIT_ARGS=()
if [[ "${LIMIT}" != "0" ]]; then
  LIMIT_ARGS=(--limit "${LIMIT}")
fi

echo "[1/5] Generating corpus via teacher (${MODEL}, limit=${LIMIT})..."
cargo run --release -p operator-synthetic-cli --bin operator-realistic-corpus -- \
  --scenarios "${SCENARIOS}" \
  --output "${OUT}" \
  --api-base "${API_BASE}" \
  --api-key-file "${API_KEY_FILE}" \
  --prompt "${PROMPT}" \
  --model "${MODEL}" \
  --temperature 0.0 \
  --max-drop-rate 0.05 \
  "${LIMIT_ARGS[@]}"

echo "[2/5] Contract coverage..."
cargo run --release -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories "${OUT}/trajectories.jsonl" \
  --require-full-coverage \
  --require-zero-invalid \
  >"${OUT}/contract-coverage.txt"

echo "[3/5] SFT prep + no-gold audit..."
python3 "${ROOT}/scripts/operator/prepare_operator_sft_dataset.py" \
  --trajectories "${OUT}/trajectories.jsonl" \
  --output "${OUT}/sft" \
  --split-mode group \
  --group-key about \
  --eval-ratio 0.15 \
  --force

python3 "${ROOT}/scripts/operator/audit_operator_sft_no_gold.py" \
  "${OUT}/sft/openai_train.jsonl" \
  "${OUT}/sft/openai_eval.jsonl" \
  --output "${OUT}/no_gold_audit.json"

echo "[4/5] Frontier ceiling..."
cargo run --release -p operator-evaluation-cli --bin operator-llm-baseline -- \
  --sft "${OUT}/sft/openai_eval.jsonl" \
  --output "${OUT}/frontier-ceiling" \
  --api-base "${API_BASE}" \
  --api-key-file "${API_KEY_FILE}" \
  --model "${MODEL}" \
  --temperature 0.0

echo "[5/5] Oracle round-trip smoke..."
OPERATOR_SMOKE_TRAJECTORIES="${OUT}/trajectories.jsonl" \
OPERATOR_SMOKE_SFT_DIR="${OUT}/sft" \
  bash "${ROOT}/scripts/operator/round_trip_smoke.sh"

echo "v7.3 realistic corpus OK: ${OUT}"
printf 'run_id:        %s\n' "${RUN_ID}"
printf 'trajectories:  %s\n' "${OUT}/trajectories.jsonl"
printf 'sft:           %s\n' "${OUT}/sft/"
printf 'report:        %s\n' "${OUT}/report.json"
printf 'coverage:      %s\n' "${OUT}/contract-coverage.txt"
printf 'ceiling:       %s\n' "${OUT}/frontier-ceiling/summary.json"
