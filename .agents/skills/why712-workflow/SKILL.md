---
name: why712-workflow
description: Continue implementation, maintenance, research, or release work in the why712 repository. Use for changes driven by IMPLEMENTATION_PLAN.md, EIP-712 or ERC behavior, Rust/alloy APIs, project documentation, CI, fixtures, CLI, WASM, or reusable agent workflows in this repository.
---

# why712 Workflow

Keep work aligned with the living plan, verified sources, and the pure-core boundary. Leave
durable context so another agent can continue without rediscovering constraints.

## Start from repository state

1. Read `AGENTS.md`.
2. For non-trivial work, read `IMPLEMENTATION_PLAN.md`, `.vault/STATUS.md`, and
   `.vault/BLOCKERS.md`.
3. Read the directly relevant file in `docs/` before inspecting external sources.
4. Inspect the working tree and preserve unrelated user changes.
5. Select the earliest incomplete plan step whose prerequisites are satisfied.

## Verify uncertain facts

- Treat repository docs and pinned source code as authoritative for local decisions.
- Inspect the exact dependency source from `Cargo.lock` before relying on remembered Rust or
  alloy APIs.
- When a technical fact is uncertain, version-sensitive, security-sensitive, or absent from
  local sources, consult current primary sources: the applicable EIP/ERC, official Rust or
  crate documentation, or the upstream repository. Do not guess from memory or rely on
  unsourced tutorials.
- Record a source and its checked date in durable docs when the conclusion affects architecture,
  security, interoperability, or future implementation work.
- Record unresolved uncertainty as a `Q-…` entry. Record a barrier as a `B-…` entry before
  attempting a workaround.

## Implement and verify

- Keep deterministic diagnosis synchronous and free of I/O, clock, and randomness. Acquire a
  plain `OnchainSnapshot` first. Concrete native I/O belongs only in `rpc.rs` behind `rpc`;
  browser I/O belongs in the WASM adapter.
- Use `alloy-dyn-abi` for EIP-712 hashing; do not implement the algorithm independently.
- Prefer the native pinned Rust toolchain. Run the development container only when the native
  toolchain or linker is unavailable.
- Run the narrowest relevant test while iterating, then finish a plan step with:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```

- Add an entry to `docs/diagnostics.md` in the same change as every `FindingCode`.

## Preserve reusable knowledge

- Put code-level facts next to the code, stable system/process knowledge in `docs/`, and
  in-flight state in `.vault/`.
- Create or update a focused skill under `.agents/skills/` when a workflow is likely to recur,
  has non-obvious ordered steps, or packages project knowledge another agent should apply
  automatically. Do not create a skill for a one-off fact or a workflow already clear in
  `AGENTS.md` or `docs/`.
- Use the `skill-creator` workflow for each new skill: initialize it, keep `SKILL.md` concise,
  add only necessary resources, and run `quick_validate.py`.
- Update `.vault/STATUS.md`, append one dated entry to `.vault/JOURNAL.md`, and update the
  status table in `IMPLEMENTATION_PLAN.md` after material progress.
