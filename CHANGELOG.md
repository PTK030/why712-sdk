# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Rebuilt the static website with Astro and TypeScript, split the WASM playground to `/app/`,
  and added searchable Starlight user documentation under `/docs/`.
- Switched new versions from MIT to PolyForm Noncommercial 1.0.0, with separate paid
  commercial licensing.
- Added Polish, Simplified Chinese, and Japanese README translations.
- Added focused agent skills for diagnostics, fixture research, and releases.
- Improved container build caching, GHCR usage guidance, and superseded-main-run cancellation.

### Added

- Pure EIP-712 parsing and intermediate computation through Alloy.
- Canonical and ERC-2098 signature parsing, recovery, and scalar diagnostics.
- Bounded root-cause search across common domain, type, encoding, and envelope mistakes.
- Stable versioned reports with text, JSON, and Markdown rendering.
- ERC-5267 and `DOMAIN_SEPARATOR()` probing through an optional rustls Alloy transport.
- Contract-signer detection and ERC-1271 ABI helpers.
- `why712 check`, `digest`, `examples`, and `explain` commands with tested exit codes.
- Shared independently verified fixture corpus, snapshots, native CI, and WASM compile guard.
- Browser WASM bindings with fetch-based RPC fallback and generated TypeScript declarations.
- Static diagnostic application with field-level findings, copied Markdown reports,
  fragment-only share links, and one-click examples from the shared fixture corpus.
- Headless browser CI and GitHub Pages deployment from the same verified static artifact.
- Responsive product introduction, project logo, and a real browser-recorded diagnostic demo.
- ERC-1271 contract-signature execution with opaque signature bytes and explicit valid, invalid,
  and unavailable outcomes.
- Non-root distroless production image and a pinned Anvil mainnet-fork Compose service.
- Multi-architecture GHCR workflow with semver/latest tags, provenance, SBOM, and GHA caching.
- Native binary, checksum, GitHub Release, and ordered crates.io publishing automation.
- Weekly Cargo/Actions updates and CI enforcement for advisories, licenses, sources, and TLS bans.

[Unreleased]: https://github.com/PTK030/why712/commits/main
