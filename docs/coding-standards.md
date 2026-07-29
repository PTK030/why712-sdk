# Coding Standards

The instinct these rules protect against: hedging. Code that covers every case instead of
deciding, layers over instead of cutting in, wraps things in `Option` in case, catches errors
it can't name, and leaves `// might need to revisit` behind. A senior engineer commits — to a
shape, a boundary, a name, a happy path, a type — and lets the rest follow. Every rule below
catches a different form of indecision.

For testing rules see [testing.md](testing.md). For the diagnostic-code catalog see
[diagnostics.md](diagnostics.md).

## Core principles

- **Zero complexity budget.** Every abstraction justifies itself with a specific, current
  benefit. A trait with one implementor is indirection, not abstraction.
- **YAGNI.** Build it when it's needed, not when it's imaginable.
- **No "while I'm at it" cleanups.** Make the change you came for. Drive-by edits hide in the
  diff and make the real change unreviewable. Put them in `.vault/BACKLOG.md` as a `T-…`.
- **No barrel modules that only re-export.** Import from the source.

## Comments

- Delete any comment whose removal loses zero information. Comments explain *why*, not *what*.
- No tutorial comments explaining the language (`// iterate over the vector`).
- No decorative dividers (`// ===== helpers =====`). Use modules to organize, not ASCII art.
- No hedging (`// should work for most cases`, `// might need to revisit`). If you're unsure,
  find out; if it's a real open question, file it as a `Q-…` in `.vault/OPEN_QUESTIONS.md`.
- No commented-out code. Git remembers.
- No `TODO: implement` stubs. If it needs to exist, write it. If it doesn't yet, don't
  declare it.
- Cryptographic and spec-derived constants get a comment naming the source — the EIP number,
  the section, the reason. `27` and `0x1626ba7e` are meaningless without it.

## Confidence: commit to a shape

- **Validate at boundaries, trust types inside.** Parsing happens in `input.rs` and at the RPC
  edge. After that, a value is what its type says it is.
- Every `?` on an `Option` and every `unwrap_or_default()` past the parse boundary is
  unconfident code. Either the boundary should have resolved it, or the type is lying.
- No defensive checks the type system already rules out.
- Optionality is a design decision, not a migration shortcut. Distinct valid states get a
  discriminated `enum`. Intentionally absent gets an explicit `Option`. Keep optionality at
  real boundaries.

## Types

- **No `unwrap()` or `expect()` in production code.** Both are `warn` at workspace level. Tests
  opt out with `#![allow(clippy::unwrap_used)]` at the top of the test module. A genuine
  exception in production needs a comment saying why the invariant holds.
- **No `as` casts to silence the compiler.** Use `try_into()` and handle the failure, or change
  the type. `as` silently truncates, and this crate deals in 256-bit integers.
- **No `unsafe`.** `unsafe_code = "forbid"` at workspace level.
- Newtypes over bare `String` and `[u8; 32]`. `B256` is not a `Digest`, and a `Digest` is not a
  `DomainSeparator` — this crate compares hashes for a living, and mixing two up would produce a
  confidently wrong diagnosis.
- **Make impossible states unrepresentable.** A discriminated `enum` beats a struct of
  `Option`s where only some combinations are legal.
- String-literal enums, not raw `String`, when the value is one of a known set.

## Errors and findings

This distinction is the most important one in the codebase.

- **An `Error` means we could not perform a diagnosis** — malformed JSON, an unparseable
  signature, an RPC that didn't answer.
- **An invalid signature is a `Finding`, not an `Error`.** It is the thing this tool exists to
  report. Returning `Err` for it would make the tool fail at exactly its purpose.

Beyond that:

- Error variants carry the fields a caller would want to read. `Error::SignatureLength { len }`,
  not `Error::Invalid(String)` — a formatted string throws away the information a caller needs
  to branch on.
- No `catch-all` mapping of unrelated failures into one variant.
- Never silently substitute a fallback for what the caller asked for. If ERC-5267 isn't
  available we fall back to `DOMAIN_SEPARATOR()` — and the report says so, because which source
  the domain came from changes how much the diagnosis can claim.

## Diagnostics

- Every `FindingCode` variant has an entry in [diagnostics.md](diagnostics.md). CI fails
  without one.
- A finding states what was expected, what was found, and what to change. "Domain mismatch" is
  not a diagnosis; "the contract does not declare `version`, your library adds it" is.
- Confidence is part of the output. A mutation that recovers the expected signer is certain
  (~2⁻¹⁶⁰ chance of coincidence) and says so. A mutation that only reproduces the domain
  separator is weaker evidence and must be worded differently.

## Determinism

The diagnosis path performs no I/O: no network, filesystem, clock, or randomness. Chain data
arrives as a snapshot the caller supplies. The optional native adapter in `rpc.rs` acquires that
snapshot before diagnosis and is excluded from the default and wasm dependency graphs.

This is not stylistic. It is what lets the CLI and the browser run the same code, what makes
every diagnosis reproducible from its inputs alone, and what keeps the crate compiling for
`wasm32-unknown-unknown`. A dependency that breaks the wasm build breaks the demo page, which
is how anyone finds this project at all.

## Dependency policy

`deny.toml` is the executable dependency boundary. CI checks the complete native and WASM graph,
including development and build dependencies, for:

- RustSec advisories, yanked releases, and unsound dependencies;
- an explicit reviewed-license allowlist;
- wildcard version requirements and unexpected Git or registry sources;
- accidental reintroduction of OpenSSL/native-tls after the deliberate rustls choice.

Duplicate transitive versions are reported but do not fail CI: Alloy's graph contains several
legitimate version families, so a blanket denial would turn upstream release cadence into local
exceptions. Direct workspace dependency duplication and unused workspace declarations do fail.
Dependabot checks Cargo and pinned GitHub Actions weekly; its pull requests still pass the same
policy and full quality gates before merge.
