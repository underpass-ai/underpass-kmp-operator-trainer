#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
RUN_ID="${OPERATOR_CONTRACT_RUN_ID:-operator-contract-v6-$(date -u +%Y%m%dT%H%M%SZ)}"
ARTIFACT_ROOT="${OPERATOR_ARTIFACT_ROOT:-${ROOT}/../rehydration-kernel-artifacts/operator}"
OUT_DIR="${OPERATOR_CONTRACT_OUT_DIR:-${ARTIFACT_ROOT}/${RUN_ID}}"
MINIMUM_EXAMPLES="${OPERATOR_CONTRACT_MINIMUM_EXAMPLES:-4}"
DATASET_ID="${OPERATOR_CONTRACT_DATASET_ID:-dataset:${RUN_ID}}"

TRAJECTORIES="${OUT_DIR}/trajectories.jsonl"
SFT_DIR="${OUT_DIR}/sft"
EVAL_GROUPS="about:ingest:0,about:wake:0,about:ask:0,about:near:0,about:goto:0,about:rewind:0,about:forward:0,about:trace:0,about:inspect:0,about:write_memory:0"

mkdir -p "${OUT_DIR}"

cargo run --quiet -p operator-synthetic-cli --bin operator-synthesize -- \
  --dataset-id "${DATASET_ID}" \
  --minimum-examples "${MINIMUM_EXAMPLES}" \
  --output "${TRAJECTORIES}"

cargo run --quiet -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories "${TRAJECTORIES}" \
  --require-full-coverage \
  --require-zero-invalid

python3 "${ROOT}/scripts/operator/prepare_operator_sft_dataset.py" \
  --trajectories "${TRAJECTORIES}" \
  --output "${SFT_DIR}" \
  --split-mode group \
  --group-key about \
  --eval-group-values "${EVAL_GROUPS}" \
  --eval-ratio 0.1 \
  --force

cargo run --quiet -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories "${SFT_DIR}/train_trajectories.jsonl" \
  --require-full-coverage \
  --require-zero-invalid

cargo run --quiet -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories "${SFT_DIR}/eval_trajectories.jsonl" \
  --require-full-coverage \
  --require-zero-invalid

python3 "${ROOT}/scripts/operator/audit_operator_sft_no_gold.py" \
  "${SFT_DIR}/openai_train.jsonl" \
  "${SFT_DIR}/openai_eval.jsonl" \
  --output "${OUT_DIR}/no_gold_audit.json"

python3 "${ROOT}/scripts/operator/train_operator_sft_lora.py" \
  --train-jsonl "${SFT_DIR}/openai_train.jsonl" \
  --eval-jsonl "${SFT_DIR}/openai_eval.jsonl" \
  --output-dir "${OUT_DIR}/train-validate-only" \
  --validate-only

python3 "${ROOT}/scripts/operator/predict_operator_sft.py" \
  --dataset-jsonl "${SFT_DIR}/eval.jsonl" \
  --model-id "contract-v6-validation-only" \
  --output "${OUT_DIR}/predict-validate-only" \
  --resolve-prepared-payloads \
  --validate-only

OPERATOR_SMOKE_TRAJECTORIES="${TRAJECTORIES}" \
OPERATOR_SMOKE_SFT_DIR="${SFT_DIR}" \
  bash "${ROOT}/scripts/operator/round_trip_smoke.sh"

printf 'operator contract v6 corpus ok\n'
printf 'run_id:       %s\n' "${RUN_ID}"
printf 'trajectories: %s\n' "${TRAJECTORIES}"
printf 'sft_dir:      %s\n' "${SFT_DIR}"
