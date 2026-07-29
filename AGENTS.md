# AGENTS.md

why712 tells you why an EIP-712 signature doesn't verify. It reads the contract's real domain
separator over RPC, recomputes the digest locally, and reports the specific cause of the
mismatch — and where it can, which change makes the signature work.

Two front-ends, one core: a CLI (`why712`) and a WASM demo page. There is no backend.

## Repository map

This is a Cargo workspace.

- `crates/why712-core` — all the logic. Pure, synchronous, no network, no clock, no randomness.
- `crates/why712-cli` — the `why712` binary. Argument parsing and rendering only.
- `crates/why712-wasm` — JS bindings and a `fetch`-based RPC transport.
- `web/` — the static demo page. Plain HTML/CSS/JS, no bundler.
- `docker/` — the shipped CLI image and the development toolchain image.
- `scripts/` — helper scripts, notably `dev-cargo.sh`.

## Docs

`docs/` is the source of truth for system-level and process-level knowledge. **"The docs" always
means this directory, not the web.** Look here before searching online; these files record the
conventions and gotchas you cannot derive from the code.

When you learn something durable — a convention, a gotcha, a piece of system context that will
outlive the current task — update a doc. Code-level facts belong in a comment next to the code.

| Doc | What's in it |
|---|---|
| [docs/diagnostics.md](docs/diagnostics.md) | Every `FindingCode`: what it means, why it happens, how to fix it |
| [docs/architecture.md](docs/architecture.md) | Crate layering, the pure-core boundary, how RPC stays out of it |
| [docs/coding-standards.md](docs/coding-standards.md) | What gets merged and what doesn't |
| [docs/testing.md](docs/testing.md) | Test vectors, fixtures, snapshots, wasm tests |
| [docs/development.md](docs/development.md) | Local setup, building for wasm, running the demo page |
| [docs/release.md](docs/release.md) | Release playbook |
| [SECURITY.md](SECURITY.md) | Threat model and what never leaves the user's machine |

## Local working context

Two paths are **untracked** (see `.gitignore`) but load-bearing. They will not appear in a fresh
clone; if they exist, they are current.

- `IMPLEMENTATION_PLAN.md` — the full plan, its milestones, and their status.
- `.vault/` — working memory: blockers, open questions, decisions, backlog, journal.
  Read `.vault/README.md` first; it defines the format and the boundary against `docs/`.

**Protocol.** Before non-trivial work, read `IMPLEMENTATION_PLAN.md`, `.vault/STATUS.md`, and
`.vault/BLOCKERS.md` — they exist so you don't rediscover a dead end someone already mapped.
When you finish, update `.vault/STATUS.md` and append to `.vault/JOURNAL.md`.

**Record a barrier as a `B-…` entry before you work around it, not after.** The workaround is
what survives in the code; the reason for it is what gets lost.

## Project skills and research

Use `.agents/skills/why712-workflow/SKILL.md` when continuing implementation, maintenance,
research, or release work in this repository.

Do not guess when a fact is absent from local docs or may have changed. Inspect the exact pinned
dependency source first, then consult current primary sources such as the applicable EIP/ERC,
official Rust or crate documentation, or the upstream repository. Promote durable conclusions
to `docs/` and record unresolved questions in `.vault/OPEN_QUESTIONS.md`.

When a non-obvious workflow will recur or would help another agent work correctly, package it
as a focused skill under `.agents/skills/` using the `skill-creator` workflow. Do not turn
one-off facts or rules already clear in `AGENTS.md` or `docs/` into skills.

## Language

Everything committed to this repository is in English, without exception: docs, comments,
identifiers, CLI output, finding text, the demo UI, templates, and commit messages.

## Quick start

With Rust installed natively:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo check -p why712-core --target wasm32-unknown-unknown
```

Without Rust, run the same commands through the pinned toolchain container:

```bash
scripts/dev-cargo.sh test --workspace
```

The wasm check is not optional busywork. `why712-core` must build for
`wasm32-unknown-unknown` or the demo page cannot exist, and a dependency that breaks it is far
cheaper to catch now than after the code is written. CI runs it on every push.

## Non-negotiables

- **`why712-core` never performs I/O.** No network, no filesystem, no clock, no randomness.
  Chain data enters as a plain snapshot the caller supplies. This is what lets the CLI and the
  browser share one implementation, and what makes every diagnosis reproducible from its inputs.
- **We do not implement EIP-712 hashing ourselves.** `alloy-dyn-abi` is the engine. For a tool
  whose entire purpose is telling people their hashing is wrong, a bug in its own hashing is the
  worst possible failure.
- **An `Error` means "we could not diagnose", never "the signature is invalid."** An invalid
  signature is a finding, and findings are the product.
- **A new `FindingCode` variant requires an entry in `docs/diagnostics.md`.** CI enforces it.
- **Scope is deliberately narrow.** No database, no backend, no wallet integration, no non-EVM
  chains, no signing. See `IMPLEMENTATION_PLAN.md` §12 before proposing a feature.
