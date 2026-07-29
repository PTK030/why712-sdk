# Contributing

Thanks for improving why712. The project values small, evidence-backed changes because it makes
claims about cryptographic inputs and must be easier to audit than the code it diagnoses.

## Before coding

- Read `AGENTS.md`, the relevant file in `docs/`, and `docs/coding-standards.md`.
- Check existing issues and keep one change focused on one problem.
- For version-sensitive Rust, Alloy, EIP, or ERC behavior, inspect the pinned source and current
  primary specification instead of relying on memory.

## Pull requests

Do:

- add or update tests that would fail without the change;
- use `alloy-dyn-abi` for EIP-712 hashing and alloy-sol-types for ABI encoding;
- add a `docs/diagnostics.md` entry in the same change as every `FindingCode`;
- update stable docs when a durable convention or interoperability fact changes;
- explain how the change was verified.

Do not:

- introduce network, filesystem, clock, or randomness into deterministic diagnosis;
- hand-roll EIP-712 hashing;
- turn an invalid signature into a library `Error`;
- include private keys, seed phrases, production RPC credentials, or user data;
- combine unrelated cleanup with the requested change;
- weaken, skip, or snapshot-accept a failing test without explaining the semantic change.

## Verify

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```

Native Rust is preferred. `scripts/dev-cargo.sh` provides a pinned Docker fallback when the
host toolchain or linker is unavailable.

## Fixtures and snapshots

The shared corpus under `crates/why712-core/fixtures/` feeds core tests, the CLI, and the future
web demo. Expected hashes and signatures must come from an independent implementation or
specification, with the source and version recorded in `docs/testing.md`.

Review snapshot diffs as API changes. `cargo insta accept` is not evidence that a new result is
correct.

## Security

Use GitHub's private vulnerability reporting path described in `SECURITY.md` for exploitable
issues. Do not open a public issue containing secrets or a proof of concept that puts users at
risk.
