#!/usr/bin/env bash
# Regenerate and verify local Operator E2E tooling before live replay runs.
# Reference: docs/operations/preflight.md

set -uo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
# shellcheck source=scripts/e2e/lib.sh
source "$SCRIPT_DIR/lib.sh"

parse_common_flags "$@"

NAMESPACE=${NAMESPACE:-underpass-runtime}
OPERATOR_POD_SELECTOR=${OPERATOR_POD_SELECTOR:-app.kubernetes.io/instance=underpass-llm-operator-qwen05}
OPERATOR_ADAPTER_PATH=${OPERATOR_ADAPTER_PATH:-/adapters/operator-v8.1.2-sft-v2/adapter_model.safetensors}
EXPECTED_ADAPTER_SHA=${EXPECTED_ADAPTER_SHA:-43186fa848c5f0e9d71915023f8f01c2341042de8aaf57b0c3c0c574a0f44379}
OPERATOR_MODELS_URL=${OPERATOR_MODELS_URL:-https://0.5b.llm.underpassai.com/v1/models}
EXPECTED_OPERATOR_MODEL_ID=${EXPECTED_OPERATOR_MODEL_ID:-operator-v8.1.2}
CLIENT_CERT=${CLIENT_CERT:-/tmp/client.crt}
CLIENT_KEY=${CLIENT_KEY:-/tmp/client.key}
EXPECTED_HELM_RELEASES=${EXPECTED_HELM_RELEASES:-underpass-runtime rehydration-kernel underpass-llm-operator-qwen05}
KMP_ENDPOINT=${KMP_ENDPOINT:-https://rehydration-kernel.underpassai.com}
REAL_ANCHOR=${REAL_ANCHOR:-article:incident:checkout-latency:20260504T233722Z:frontend}
OPERATOR_SESSION_RUN_BIN=${OPERATOR_SESSION_RUN_BIN:-operator_session_run}

if [[ "$ALLOW_REDEPLOY" == "1" ]]; then
  warn "redeploy flag" "--redeploy accepted but this script does not run Helm mutations yet"
fi
if [[ "$ALLOW_REINSTALL_ADAPTERS" == "1" ]]; then
  warn "adapter reinstall flag" "operator repo has no adapter installer; run underpass-vllms script manually if needed"
fi

check_branch_state "$REPO_ROOT" "git state"
run_checked "cargo release build" "workspace release build completed" cargo build --workspace --release
run_checked "cargo install operator" "operator-runtime-cli installed to user cargo bin" cargo install --path crates/operator-runtime-cli --force
check_binary_freshness "$OPERATOR_SESSION_RUN_BIN" "$REPO_ROOT" "operator-session-run freshness"
check_adapter_sha "$NAMESPACE" "$OPERATOR_POD_SELECTOR" "$OPERATOR_ADAPTER_PATH" "$EXPECTED_ADAPTER_SHA" "operator adapter sha"
check_endpoint_model "$OPERATOR_MODELS_URL" "$CLIENT_CERT" "$CLIENT_KEY" "$EXPECTED_OPERATOR_MODEL_ID" "operator /v1/models"
check_mtls_cert "$CLIENT_CERT" "client cert validity"

if [[ -s /tmp/openai.txt && -s /tmp/claude.txt ]]; then
  ok "api key files" "/tmp/openai.txt and /tmp/claude.txt are present and non-empty"
else
  fail "api key files" "expected non-empty /tmp/openai.txt and /tmp/claude.txt"
fi

if helm list -n "$NAMESPACE" >/tmp/operator-e2e-helm-list.txt 2>&1; then
  ok "helm list" "helm list succeeded for namespace $NAMESPACE"
  if [[ "$VERBOSE" == "1" ]]; then
    cat /tmp/operator-e2e-helm-list.txt
  fi
else
  fail "helm list" "helm list failed: $(cat /tmp/operator-e2e-helm-list.txt)"
fi
for release in $EXPECTED_HELM_RELEASES; do
  check_helm_release_revision "$release" "$NAMESPACE" 1 "helm release $release"
done

if command -v rehydration-mcp >/dev/null 2>&1; then
  request=$(printf '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"kernel_wake","arguments":{"about":"%s"}}}' "$REAL_ANCHOR")
  if response=$(printf '%s\n' "$request" | REHYDRATION_MCP_BACKEND=grpc REHYDRATION_KERNEL_GRPC_ENDPOINT="$KMP_ENDPOINT" rehydration-mcp 2>&1); then
    if printf '%s' "$response" | grep -Fq '"isError":false'; then
      ok "rehydration-mcp smoke" "kernel_wake succeeded for $REAL_ANCHOR"
    else
      fail "rehydration-mcp smoke" "kernel_wake did not return isError=false: $response"
    fi
  else
    fail "rehydration-mcp smoke" "rehydration-mcp command failed: $response"
  fi
else
  fail "rehydration-mcp smoke" "rehydration-mcp not on PATH; run kernel regen first"
fi

finish_summary
