---
name: add-diagnostic
description: Add or change a why712 FindingCode and carry the diagnostic contract through the pure core, catalog, fixtures, tests, CLI, and WASM surfaces. Use when implementing a new mismatch cause, changing finding behavior, or extending the hypothesis engine.
---

# Add a why712 diagnostic

1. Read `IMPLEMENTATION_PLAN.md`, `.vault/STATUS.md`, `.vault/BLOCKERS.md`,
   `docs/diagnostics.md`, `docs/architecture.md`, and `docs/testing.md`.
2. Confirm the behavior from the exact pinned dependency source, then current primary
   EIP/ERC or upstream documentation. Do not infer cryptographic behavior from memory.
3. Preserve the pure diagnosis boundary: acquire chain evidence before diagnosis and pass it
   through `OnchainSnapshot`. Never add I/O, time, or randomness to the engine.
4. Add or change the stable `FindingCode`, structured evidence, explanation, and concrete fix
   together. An invalid signature is a finding, not an `Error`.
5. Update `docs/diagnostics.md` in the same change. Add an independently verified fixture when
   the behavior is representable as a reusable case; update its manifest and provenance.
6. Exercise both positive and nearby negative cases. Update snapshots intentionally and inspect
   their complete diff.
7. Check CLI text/JSON/Markdown and WASM serialization when the public report changes.
8. Run format, workspace clippy with all targets/features, workspace tests, and the core wasm
   check. Run browser tests when bindings or browser-visible behavior changed.
9. Promote durable conclusions to `docs/`; record uncertainty in `.vault/OPEN_QUESTIONS.md`.
   Update `.vault/STATUS.md` and append `.vault/JOURNAL.md`.

Reject unbounded mutation searches, custom EIP-712 hashing, silent signature normalization,
and findings without a user-actionable fix.
