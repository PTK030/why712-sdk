# syntax=docker/dockerfile:1

FROM rust:1.97-slim-bookworm AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY docs/diagnostics.md ./docs/diagnostics.md

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/workspace/target,sharing=locked \
    cargo build --locked --release -p why712 && \
    cp /workspace/target/release/why712 /tmp/why712

FROM gcr.io/distroless/cc-debian12:nonroot

LABEL org.opencontainers.image.title="why712" \
      org.opencontainers.image.description="Diagnose EIP-712 signature verification failures" \
      org.opencontainers.image.source="https://github.com/PTK030/why712" \
      org.opencontainers.image.licenses="PolyForm-Noncommercial-1.0.0"

COPY --from=builder --chown=nonroot:nonroot /tmp/why712 /usr/local/bin/why712

ENTRYPOINT ["/usr/local/bin/why712"]
CMD ["--help"]
