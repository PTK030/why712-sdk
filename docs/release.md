# Release

why712 follows semantic versioning while pre-1.0 minor versions identify product milestones:

- `0.1.x`: native core and CLI;
- `0.2.x`: WASM bindings and browser demo;
- `0.3.x`: ERC-1271 execution, production image, and automated distribution.

Container publishing is automated; binary, GitHub Release, and crates.io publishing remain a
checklist until the next implementation step lands.

## Container publishing

`.github/workflows/docker.yml` builds the production image for Linux AMD64 and ARM64 on relevant
pull requests and merge queues. It publishes only for pushes:

- `main` updates `ghcr.io/ptk030/why712:latest` and a `sha-...` traceability tag;
- a stable `vX.Y.Z` tag publishes `X.Y.Z`, `X.Y`, and `latest`;
- provenance and an SBOM are attached by BuildKit;
- layer cache is stored in GitHub Actions, not in the runtime image.

The job defaults to read-only repository access and grants `packages: write` only to the image
job. GHCR authentication uses the scoped `GITHUB_TOKEN`; no registry password or PAT is stored
as a repository secret. Every third-party action is pinned to a full commit SHA resolved from
its official release tag (last checked 2026-07-29).

Before tagging a release, confirm the image built from the same commit passes:

```bash
docker run --rm --network none --read-only ghcr.io/ptk030/why712:X.Y.Z --version
```

After publishing, inspect the manifest for both architectures, verify the OCI source/license
labels and attestations, then run one matching and one mismatching fixture by digest-pinned image
reference. If an image is broken, preserve the immutable version tag, document the incident, and
publish a fixed patch version; move `latest` only to that verified patch.

## Prepare

1. Start from a clean `main` checkout and review all changes since the previous tag.
2. Set the same version in the workspace package metadata.
3. Move user-visible entries from `Unreleased` in `CHANGELOG.md` into a dated release section.
4. Confirm README status and installation instructions match what will actually ship.
5. Run the complete quality gate:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```

## Package verification

Inspect the exact files Cargo will publish:

```bash
cargo package -p why712-core --list
cargo package -p why712 --list
cargo publish -p why712-core --dry-run
cargo publish -p why712 --dry-run
```

The CLI depends on the published core version, so publish `why712-core` first. Wait until the
registry index resolves it before publishing `why712`.

Do not publish if package contents include fixtures containing secrets, local plans, `.vault/`,
`.obsidian/`, build outputs, or machine-specific files. The Anvil key documented in test
fixtures is public test-vector material, not a production secret.

## Tag and verify

Create an annotated `vX.Y.Z` tag only after package and artifact checks pass. Release notes
should come from the matching changelog section and identify any diagnostic-schema change.

After publishing:

- install into a clean temporary Cargo home and run `why712 --version`;
- run one matching and one mismatching bundled fixture;
- verify exit codes 0, 1, and 2;
- verify the crates.io package links, README, license, and repository URL.

When release workflows and container publishing land, add their artifact, provenance, image,
and rollback checks here before enabling tags in automation.

## Recovery

Crates cannot be overwritten. Yank a broken version, document why in the changelog, fix forward,
and publish a new patch version. Do not move or recreate an existing release tag.
