# Operator runtime image — runs the single-step session CLI as a Kubernetes Job
# (no host binaries; everything executes in-cluster). Rootless by construction:
# the final stage runs as a non-root system user.
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
    -p operator-runtime-cli --bin operator_session_run

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /home/appuser --shell /usr/sbin/nologin appuser

COPY --from=builder /workspace/target/release/operator_session_run /usr/local/bin/operator-session-run

# Rootless runtime: never run as root (avoid the legacy debian `operator` group).
USER appuser

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/operator-session-run"]
