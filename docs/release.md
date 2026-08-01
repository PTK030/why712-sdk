# Release

why712 follows semantic versioning while pre-1.0 minor versions identify product milestones:

- `0.1.x`: native core and CLI;
- `0.2.x`: Rust/WebAssembly TypeScript SDK;
- `0.3.x`: stateful protocol diagnosis, production image, and automated distribution.

Container, binary, GitHub Release, crates.io, and npm publishing are automated. The repository and
registry authentication described below still require one-time owner configuration.

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

## Binary and crate publishing

Pushing a stable `vX.Y.Z` tag starts `.github/workflows/release.yml`. Before any irreversible
operation, the workflow requires:

- the tag to have the exact stable-semver shape `vX.Y.Z`;
- `why712-core` and `why712` to both have version `X.Y.Z`;
- the npm package under `packages/why712-sdk` to have version `X.Y.Z`;
- `CHANGELOG.md` to contain a matching `## [X.Y.Z]` section.

Four native runners build `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`. Unix archives are `.tar.gz`; Windows is
`.zip`. Each contains the binary, README, and license. The GitHub Release also contains a
verified `SHA256SUMS` manifest. Its curated notes come from the matching changelog section, with
GitHub's generated comparison appended. Release creation gets `contents: write` only in its
final job.

The crates.io job publishes `why712-core`, waits up to ten minutes for that exact version to
appear in the registry API/index, then verifies and publishes the `why712` CLI. It detects
already-published versions so a run can safely continue after a partial failure.

The npm job builds both conditional WASM exports, type-checks the TypeScript facade, runs the
Node contract tests, extracts and imports the exact tarball, then publishes `why712`. It also
detects an already-published version so a release can resume after partial failure.

### One-time crates.io authentication

Trusted publishing cannot claim a new crate name, so the first release needs a crates.io API
token:

1. Create a protected GitHub environment named `release`; restrict it to protected version tags
   and require an owner approval.
2. Add an environment secret named `CRATES_IO_TOKEN`. For the bootstrap release it must be
   allowed to create the two new crates. Remove it after both packages exist.
3. On crates.io, configure a trusted publisher for **each** crate with repository
   `PTK030/why712`, workflow `release.yml`, and environment `release`.
4. Set the GitHub environment variable `CRATES_IO_TRUSTED_PUBLISHING=true`.

The official crates.io authentication action then exchanges GitHub OIDC identity for a
short-lived token and revokes it after the job. The publish job alone receives
`id-token: write`; no long-lived token is needed after bootstrap.

### One-time npm authentication

The public npm name `why712` returned `404 Not Found` on 2026-08-01 and was available before the
SDK package was prepared. The first publish must claim it with an npm account token stored as
the `NPM_TOKEN` secret in the protected `release` environment. After the package exists:

1. Configure its npm trusted publisher for GitHub owner `PTK030`, repository `why712`, workflow
   `release.yml`, and environment `release`.
2. Remove `NPM_TOKEN`; the release job already grants only `contents: read` and `id-token: write`.
3. Restrict traditional token publishing in the npm package settings after one trusted publish
   succeeds.

Official [npm trusted-publisher documentation](https://docs.npmjs.com/trusted-publishers/) was
checked on 2026-08-01. It requires a GitHub-hosted runner, matching repository metadata, the
workflow filename, and OIDC `id-token: write`. Trusted publishing automatically emits provenance
for a public package from a public repository.

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
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
```

## Package verification

Inspect the exact files Cargo will publish:

```bash
cargo package -p why712-core --list
cargo package -p why712 --list
cargo publish -p why712-core --dry-run
cargo publish -p why712 --dry-run
pnpm --filter why712 pack --pack-destination target/npm
node scripts/verify-sdk-package.mjs target/npm/why712-X.Y.Z.tgz
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
- install the npm tarball into a clean Node project and run one `diagnoseOffline()` call;
- verify the npm package exposes both Node and browser conditions and includes both WASM files.

For the GitHub Release, download every archive, run `sha256sum --check SHA256SUMS`, inspect the
archive contents, and execute the binary on its matching platform. For GHCR, also perform the
manifest, provenance, SBOM, and digest-pinned checks in "Container publishing".

## Recovery

Crates cannot be overwritten. Yank a broken version, document why in the changelog, fix forward,
and publish a new patch version. Do not move or recreate an existing release tag.
