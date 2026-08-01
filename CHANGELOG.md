# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Repositioned why712 as an embeddable developer tool: a typed npm SDK backed by the existing
  Rust/WebAssembly engine, plus the native CLI and Rust crate.
- Removed the Astro landing page, browser playground, Starlight site, and GitHub Pages deployment,
  then updated every translated README for the SDK product.
- Licensed the complete Rust, TypeScript/WASM, CLI, and distribution surface under MIT for
  unrestricted open-source and commercial adoption subject to the license notice.
- Added focused agent skills for diagnostics, fixture research, and releases.
- Added a documentation-sync skill and PR contract so features, public behavior changes, and
  releases update affected docs in the same change.
- Improved container build caching, GHCR usage guidance, and superseded-main-run cancellation.

### Added

- Report schema v2 context, `eth_signTypedData_v4` input, block timestamps, stateful EIP-2612
  and Permit2 findings, read-only EIP-2612 simulation, and ERC-6492 wrapper detection.
- Direct transaction-hash diagnosis for allowlisted EIP-2612 and Permit2 calls, with mined
  parent-block or pending latest-state reads, exact Permit2 calldata simulation, decoded
  ERC-6492 wrapper context, and strict unknown-selector refusal.

- Pure EIP-712 parsing and intermediate computation through Alloy.
- Canonical and ERC-2098 signature parsing, recovery, and scalar diagnostics.
- Bounded root-cause search across common domain, type, encoding, and envelope mistakes.
- Stable versioned reports with text, JSON, and Markdown rendering.
- ERC-5267 and `DOMAIN_SEPARATOR()` probing through an optional rustls Alloy transport.
- Contract-signer detection and ERC-1271 ABI helpers.
- `why712 check`, `digest`, `examples`, and `explain` commands with tested exit codes.
- Shared independently verified fixture corpus, snapshots, native CI, and WASM compile guard.
- Browser and Node WASM bindings over `globalThis.fetch`, wrapped by conditional npm exports,
  stable TypeScript declarations, Node contract tests, and extracted-tarball verification.
- ERC-1271 contract-signature execution with opaque signature bytes and explicit valid, invalid,
  and unavailable outcomes.
- Non-root distroless production image and a pinned Anvil mainnet-fork Compose service.
- Multi-architecture GHCR workflow with semver/latest tags, provenance, SBOM, and GHA caching.
- Native binary, checksum, GitHub Release, and ordered crates.io publishing automation.
- Weekly Cargo/Actions updates and CI enforcement for advisories, licenses, sources, and TLS bans.

[Unreleased]: https://github.com/PTK030/why712/commits/main
