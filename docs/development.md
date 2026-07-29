# Development

## Native toolchain

Native Rust is the primary development path. `rust-toolchain.toml` pins Rust 1.97, rustfmt,
Clippy, and `wasm32-unknown-unknown`; rustup selects them automatically inside the repository.

Verify the installation:

```bash
cargo --version
rustc --version
rustup target list --installed
```

On Windows, the MSVC target also needs the Visual Studio C++ linker. A working native Windows
setup has been verified for this repository.

## Container fallback

If Rust or the native linker is unavailable, use the pinned development image:

```bash
scripts/dev-cargo.sh test --workspace --all-features
```

The script builds `docker/dev.Dockerfile`, keeps the Cargo registry and Linux target directory
in named Docker volumes, and does not mix container build artifacts into the host `target/`.
Docker Desktop must be running on Windows.

## Build and run

```bash
cargo build --workspace
cargo run -p why712 -- --help
cargo run -p why712 -- examples
```

A useful offline smoke test is:

```bash
cargo run -p why712 -- check \
  --typed-data crates/why712-core/fixtures/examples/valid-mail.json \
  --signature 0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c \
  --signer 0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826 \
  --offline
```

Use `--typed-data -` to read JSON from stdin. Use `--rpc` together with `--contract` for live
enrichment; set `--offline` when a command must not make a network request.

## Required checks

Run all four before a commit that completes a plan step:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```

The last check uses the default core feature set. It proves the optional native provider stack
has not leaked into the browser dependency graph.

## Repository knowledge

`docs/` is stable public knowledge. `IMPLEMENTATION_PLAN.md` and `.vault/` are local working
state and are intentionally gitignored. Agents continuing project work must follow
`AGENTS.md` and `.agents/skills/why712-workflow/SKILL.md`: inspect pinned dependency source,
consult current primary documentation when uncertain, and preserve reusable workflows as
focused project skills.

The WASM demo and `wasm-pack` workflow are not implemented yet. Do not add ad hoc browser build
steps before the corresponding implementation-plan milestone.
