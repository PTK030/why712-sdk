<p align="center"><img src="assets/logo.svg" width="320" height="116" alt="why712 terminal wordmark"></p>

<h1 align="center">why712</h1>

<p align="center">
  English · <a href="README.pl.md">Polski</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.ja.md">日本語</a>
</p>

<p align="center">A Rust-powered TypeScript SDK that explains why an EIP-712 signature does not verify.</p>

<p align="center">
  <a href="https://github.com/PTK030/why712/actions/workflows/ci.yml"><img src="https://github.com/PTK030/why712/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
</p>

why712 is an embeddable developer tool for EIP-712, EIP-2612, Permit2, ERC-1271, and ERC-6492.
Its deterministic diagnosis engine is written in Rust and ships inside the TypeScript package as
WebAssembly. It finds specific root causes such as the wrong chain ID, domain shape, field order,
integer width, signature encoding, nonce state, or contract-signature result.

There is no hosted application and no why712 backend. Use the SDK in application code, tests,
scripts, and CI, or use the companion CLI during interactive debugging.

## TypeScript SDK

After the first npm release:

```bash
pnpm add why712
```

```ts
import { diagnoseOffline } from "why712";

const report = await diagnoseOffline({
  typedData,
  signature,
  signer: expectedSigner,
});

for (const finding of report.findings) {
  console.error(finding.code, finding.remediation);
}
```

Use RPC enrichment when the diagnosis should compare the request with contract and protocol
state:

```ts
import { diagnose } from "why712";

const report = await diagnose({
  typedData,
  signature,
  signer: expectedSigner,
  contract: verifyingContract,
  rpcUrl: process.env.ETH_RPC_URL,
});
```

The same package has conditional browser and Node.js exports. Both execute the same Rust core;
the TypeScript facade hides WebAssembly initialization and exposes camelCase input and report
types. Modern Node.js uses its global `fetch`. A browser RPC endpoint must permit CORS.

## Build the SDK from source

Rust 1.97, Node.js 24.16.0, pnpm 11.9.0, and wasm-pack 0.15.0 are pinned by the repository.

```bash
pnpm install --frozen-lockfile
pnpm build:sdk
pnpm check:sdk
pnpm test:sdk
pnpm --filter why712 pack --pack-destination target/npm
node scripts/verify-sdk-package.mjs target/npm/why712-0.1.0.tgz
```

The publishable package is [`packages/why712-sdk`](packages/why712-sdk). Generated browser and
Node artifacts live under its ignored `dist/` directory.

## CLI

Run the companion CLI from source:

```bash
cargo run -p why712 -- check \
  --typed-data crates/why712-core/fixtures/examples/wrong-chain-id.json \
  --signature 0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c \
  --signer 0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826 \
  --chain-id 1 \
  --offline
```

After the first crates.io release:

```bash
cargo install why712
```

```text
why712 check    Diagnose a signature, optionally enriched through RPC
why712 digest   Print canonical EIP-712 intermediate hashes
why712 examples List the bundled reproducible cases
why712 explain  Explain a stable FindingCode
```

## Rust library

`why712-core` is the transport-neutral engine for Rust consumers. `diagnose_offline()` is pure
and synchronous. Callers that need chain evidence first acquire a plain `OnchainSnapshot`, then
pass it across the deterministic boundary.

The core does not implement EIP-712 hashing itself. It delegates hashing to pinned Alloy
libraries and verifies behavior against vectors produced independently with Viem, ethers, and
the applicable EIP/ERC specifications.

## Safety and scope

why712 does not sign messages, accept private keys, submit transactions, operate a wallet, or
support non-EVM chains. Never provide a private key or seed phrase. Typed data and reports may
contain sensitive business data; review them before logging or sharing them.

RPC mode sends only the public calls needed to collect chain evidence to the endpoint selected
by the caller. Offline diagnosis performs no network I/O.

## Documentation

| Document | Contents |
|---|---|
| [Diagnostic catalog](docs/diagnostics.md) | Every stable finding code, meaning, and fix |
| [Architecture](docs/architecture.md) | Rust core, TypeScript/WASM boundary, snapshots, and RPC |
| [Development](docs/development.md) | Toolchain, SDK build, CLI, and required checks |
| [Testing](docs/testing.md) | Independent vectors, fixtures, snapshots, WASM, and package tests |
| [Security](SECURITY.md) | Trust boundaries and sensitive data |
| [Release](docs/release.md) | Binary, crates.io, npm, and container release process |

## License

Open source under the [MIT License](LICENSE). You may use, modify, distribute, sublicense, and
sell copies subject to the license notice.
