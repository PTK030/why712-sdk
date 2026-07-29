#!/usr/bin/env sh
# Run a cargo command inside the pinned toolchain container.
#
# For contributors who have Rust installed natively, this script is unnecessary — just run
# cargo directly. It exists so the project can be built and tested on a machine with no Rust
# toolchain, and so everyone compiles against the exact version in rust-toolchain.toml.
#
#   scripts/dev-cargo.sh fmt --all -- --check
#   scripts/dev-cargo.sh clippy --workspace --all-targets -- -D warnings
#   scripts/dev-cargo.sh test --workspace
#
# The registry and target directories live in named volumes rather than on the host: it keeps
# Linux build artifacts out of the working tree and avoids the bind-mount I/O penalty that
# makes container builds crawl on Windows and macOS.

set -eu

IMAGE=why712-dev
ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)

# Git Bash / MSYS rewrites anything that looks like a POSIX path into a Windows path before
# the process sees it, which turns `-w /workspace` into `C:/Program Files/Git/workspace` and
# leaves `/c/Users/...` as a mount source Docker cannot resolve. Disable the rewriting and
# hand Docker a native Windows path instead.
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    MSYS_NO_PATHCONV=1
    MSYS2_ARG_CONV_EXCL='*'
    export MSYS_NO_PATHCONV MSYS2_ARG_CONV_EXCL
    ROOT=$(cd -- "$ROOT" && pwd -W 2>/dev/null || echo "$ROOT")
    ;;
esac

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "building $IMAGE ..." >&2
  docker build -f "$ROOT/docker/dev.Dockerfile" -t "$IMAGE" "$ROOT"
fi

exec docker run --rm \
  -v "$ROOT:/workspace" \
  -v why712-cargo-registry:/usr/local/cargo/registry \
  -v why712-target:/workspace/target \
  -w /workspace \
  "$IMAGE" cargo "$@"
