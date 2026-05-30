# Operator runtime image — runs the operator runtime CLIs as Kubernetes Jobs
# (no host binaries; everything executes in-cluster). Rootless by construction:
# the final stage runs as a non-root system user. Ships the single-step session
# runner and the window-expansion dataset generator; a Job picks the binary via
# its `command`.
#
# Build (rootless, e.g. buildah): buildah bud -t <ref> -f Dockerfile .
FROM rust:1.90.0-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY api ./api
COPY crates ./crates

RUN cargo build --locked --release \
    -p operator-runtime-cli \
    --bin operator_session_run \
    --bin operator_generate_window_expansions

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /home/appuser --shell /usr/sbin/nologin appuser

COPY --from=builder /workspace/target/release/operator_session_run /usr/local/bin/operator-session-run
COPY --from=builder /workspace/target/release/operator_generate_window_expansions /usr/local/bin/operator-generate-window-expansions

# Rootless runtime: never run as root (avoid the legacy debian `operator` group).
USER appuser

# Default to the session runner; window-expansion generation Jobs override
# `command` with /usr/local/bin/operator-generate-window-expansions.
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/operator-session-run"]
