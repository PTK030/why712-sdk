# Development

## Native toolchain

Native Rust is the primary development path. `rust-toolchain.toml` pins Rust 1.97, rustfmt,
Clippy, and `wasm32-unknown-unknown`; rustup selects them automatically inside the repository.

Verify the installation:

```bash
cargo --version
rustc --version
rustup target list --installed
```

On Windows, the MSVC target also needs the Visual Studio C++ linker. A working native Windows
setup has been verified for this repository.

`.mise.toml` pins Node.js 24.16.0 LTS and `wasm-pack` 0.15.0. `package.json` pins pnpm
11.9.0, while `pnpm-lock.yaml` pins the TypeScript compiler used for the public SDK facade.
Install the exact toolchain and dependencies with:

```bash
mise install
corepack enable
corepack prepare pnpm@11.9.0 --activate
pnpm install --frozen-lockfile
cargo install wasm-pack --version 0.15.0 --locked
```

## Container fallback

If Rust or the native linker is unavailable, use the pinned development image:

```bash
scripts/dev-cargo.sh test --workspace --all-features
```

The script builds `docker/dev.Dockerfile`, keeps the Cargo registry and Linux target directory
in named Docker volumes, and does not mix container build artifacts into the host `target/`.
Docker Desktop must be running on Windows.

## Production image and mainnet fork

The development image above is intentionally separate from the shipped CLI image. Build and
smoke-test the latter with:

```bash
docker build --file docker/Dockerfile --tag why712:local .
docker run --rm --network none --read-only why712:local --help
```

The multi-stage build compiles with `cargo build --locked` and copies only the release binary
into `gcr.io/distroless/cc-debian12:nonroot`. The exact runtime and input/output contract live in
[`docker/README.md`](../docker/README.md).

For real mainnet contract probes through a local endpoint:

```bash
MAINNET_RPC_URL=https://your-endpoint.example docker compose up --wait anvil
```

The service binds Anvil only to `127.0.0.1:8545`, pins Foundry 1.7.1 and chain ID 1, and deploys
no custom contracts. Stop it with `docker compose down`.

Container sources checked 2026-07-29:

- the official [distroless image catalog](https://github.com/GoogleContainerTools/distroless)
  lists `cc-debian12:nonroot`, documents its multi-architecture support, and requires vector-form
  entrypoints because no shell is present;
- the official [Foundry release](https://github.com/foundry-rs/foundry/releases/tag/v1.7.1)
  and [Anvil documentation](https://getfoundry.sh/anvil/) define the pinned fork runner and
  `--fork-url` behavior.

## Build and run

```bash
cargo build --workspace
cargo run -p why712 -- --help
cargo run -p why712 -- examples
```

A useful offline smoke test is:

```bash
cargo run -p why712 -- check \
  --typed-data crates/why712-core/fixtures/examples/valid-mail.json \
  --signature 0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c \
  --signer 0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826 \
  --offline
```

Use `--typed-data -` to read JSON from stdin. Use `--rpc` together with `--contract` for live
enrichment; set `--offline` when a command must not make a network request.

## Required checks

Run all four before a commit that completes a plan step:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```

The last check uses the default core feature set. It proves the optional native provider stack
has not leaked into the JavaScript SDK dependency graph.

For SDK-facing changes, also run:

```bash
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
pnpm --filter why712 pack --pack-destination target/npm
node scripts/verify-sdk-package.mjs target/npm/why712-0.1.0.tgz
```

## TypeScript SDK and WASM tests

Build both conditional exports of the npm package `why712`:

```bash
pnpm build:sdk
```

The build compiles `crates/why712-wasm` with wasm-pack's `web` and `nodejs` targets, removes the
generated nested ignore markers so the engine is included in the parent npm archive, and then
compiles the TypeScript facade into `packages/why712-sdk/dist`. The directory is generated and
gitignored.

Run the Node package tests and TypeScript checks with:

```bash
pnpm check:sdk
pnpm test:sdk
```

Run the JavaScript contract tests in headless Chrome:

```bash
wasm-pack test --headless --chrome crates/why712-wasm
```

`wasm-pack` 0.15.0 resolves the newest stable ChromeDriver without inspecting the installed
Chrome version on Windows. If the browser has not updated to the same major version, select a
matching driver through the official Chrome for Testing metadata and pass it explicitly:

```bash
wasm-pack test --headless --chrome --chromedriver /path/to/chromedriver crates/why712-wasm
```

ChromeDriver must match Chrome's `MAJOR.MINOR.BUILD`; patch numbers may differ.

Rust 1.97 emits standardized bulk-memory and non-trapping float-to-int instructions. The
release profile in `crates/why712-wasm/Cargo.toml` explicitly enables those features during
wasm-opt validation; removing those flags breaks the production build even though compilation
and current browsers support the module.

`.github/workflows/sdk.yml` runs the headless Chrome contract suite, builds both SDK exports,
type-checks the facade, runs Node tests, creates the npm archive, and imports the Rust engine
from the extracted archive. GitHub actions are pinned to immutable commit SHAs; update the
version comment and SHA together after checking the action's official release.

The browser export contacts only the RPC URL supplied by the SDK caller. Public RPC behavior
can change and browser endpoints may reject CORS, so fetch failures remain report evidence and
offline diagnosis continues. Never add a project proxy.

## Repository knowledge

`docs/` is stable public knowledge. `IMPLEMENTATION_PLAN.md` and `.vault/` are local working
state and are intentionally gitignored. Agents continuing project work must follow
`AGENTS.md` and `.agents/skills/why712-workflow/SKILL.md`: inspect pinned dependency source,
consult current primary documentation when uncertain, and preserve reusable workflows as
focused project skills.
