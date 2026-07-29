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
11.9.0, while `pnpm-lock.yaml` pins Astro, Starlight, TypeScript, and their complete graph.
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
has not leaked into the browser dependency graph.

For web-facing changes, also run:

```bash
pnpm test:web
pnpm check:web
pnpm build
```

## WASM bindings and browser tests

Build the complete static site. The script runs the release WASM build, writes its ignored
package and fixtures under `web/public/app/`, then builds the Astro landing page, vanilla
playground, and Starlight documentation into ignored `web/dist/`:

```bash
pnpm build
node scripts/serve-web.mjs
```

Open `http://127.0.0.1:4173/why712/`. Set `WHY712_WEB_PORT` to use another port. The server is
for local development only; production is the exact static `web/dist` Pages artifact.

Run the URL-fragment and Astro selector contract tests with:

```bash
pnpm test:web
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

`.github/workflows/wasm.yml` runs the static contract tests, headless Chrome suite, and release
build for relevant pull requests. On `main`, that same verified `web/` artifact is deployed to
GitHub Pages when the repository is public. Private repositories on plans without Pages skip
only the deploy job, keeping the verified build meaningful and green; making the repository
public activates deployment without another workflow change. GitHub actions are pinned to
immutable commit SHAs; update the version comment and SHA together after checking the action's
official release.

The demo pre-fills `https://ethereum-rpc.publicnode.com` for optional chain evidence. A real
cross-origin browser probe on 2026-07-29 confirmed that endpoint and
`https://cloudflare-eth.com`; `https://eth.llamarpc.com` failed the same probe at that time.
Public RPC behavior can change, so a fetch failure is reported as a possible CORS restriction
and diagnosis continues offline. Never add a project proxy: the browser and CLI must contact
only the endpoint selected by the user.

## Repository knowledge

`docs/` is stable public knowledge. `IMPLEMENTATION_PLAN.md` and `.vault/` are local working
state and are intentionally gitignored. Agents continuing project work must follow
`AGENTS.md` and `.agents/skills/why712-workflow/SKILL.md`: inspect pinned dependency source,
consult current primary documentation when uncertain, and preserve reusable workflows as
focused project skills.
