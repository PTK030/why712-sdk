# Development toolchain image.
#
# This is NOT the shipped CLI image — see docker/Dockerfile for that one. This image exists
# so the project can be built, linted, and tested without installing Rust on the host, and so
# every contributor compiles against the exact toolchain pinned in rust-toolchain.toml.
#
# Build:  docker build -f docker/dev.Dockerfile -t why712-dev .
# Use:    scripts/dev-cargo.sh clippy --workspace --all-targets -- -D warnings

FROM rust:1.97-slim

# The base image ships the `minimal` profile: no clippy, no rustfmt, and only the host target.
#
# Install the toolchain under the exact name rust-toolchain.toml asks for ("1.97"). The image's
# own toolchain is named differently, so rustup would otherwise treat the pin as a missing
# toolchain and re-download six components on every single container run.
RUN rustup toolchain install 1.97 \
      --profile minimal \
      --component clippy --component rustfmt \
      --target wasm32-unknown-unknown \
 && rustup default 1.97

WORKDIR /workspace
