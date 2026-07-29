---
name: release-why712
description: Prepare, publish, monitor, and verify a why712 release across crates.io, GitHub releases, GHCR, and the browser demo. Use for version bumps, release candidates, tags, publication recovery, or release audits.
---

# Release why712

Treat `docs/release.md` as the executable playbook; do not duplicate or override it here.

1. Read the implementation plan, vault status/blockers, `docs/release.md`, changelog, and all
   release workflows. Confirm the owner-controlled registry and repository prerequisites.
2. Determine the SemVer impact from public Rust APIs, report schema, finding behavior, CLI, and
   WASM bindings. Never choose a major release without explicit owner approval.
3. Verify the license metadata and packaged license files match the current repository policy.
4. Run every preflight in `docs/release.md`, including clean package inspection, all-feature
   clippy/tests, wasm compilation/browser checks, container smoke tests, and dry-run publishes.
5. Prepare the version and changelog on a release branch. Do not tag from a dirty tree or bypass
   a failing check.
6. Publish in the documented dependency order. Treat tags, crates.io publication, GitHub
   Releases, and image pushes as irreversible external actions and obtain any authority the
   current task has not granted.
7. Babysit workflows to terminal state. Verify release checksums, archives, crate metadata,
   multi-architecture GHCR manifests, provenance/SBOM, and the browser demo.
8. Use the recovery section instead of deleting or rewriting published artifacts. Update vault
   status and append the journal with exact versions, digests, and links.

Stop before publication when registry ownership, tokens, plan limitations, or an unresolved
release blocker prevents verification.
