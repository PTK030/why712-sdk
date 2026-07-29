<h1 align="center">why712</h1>

<p align="center">
  <a href="https://github.com/PTK030/why712/actions/workflows/ci.yml"><img src="https://github.com/PTK030/why712/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/why712"><img src="https://img.shields.io/crates/v/why712.svg" alt="crates.io"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT"></a>
</p>

<p align="center">Find out why your EIP-712 signature doesn't verify. One command, one concrete answer.</p>

> **Status: pre-release.** The core and CLI are being built (`v0.1.0`). The browser demo lands
> in `v0.2.0`; published binaries and the Docker image in `v0.3.0`.

- **Reads the real domain separator** from the contract via `eip712Domain()` (ERC-5267),
  falling back to `DOMAIN_SEPARATOR()`. No guessing what the contract thinks it signed.
- **Finds the root cause, not just a diff.** Searches a bounded space of likely mistakes and
  tells you which one reproduces the contract's separator or recovers your signer.
- **Works offline.** Everything except one RPC call is local, and most diagnoses need no
  network at all.
- **CI-friendly.** Exit code 1 on mismatch, `--format json`, and a Docker image so you don't
  need Rust installed.
- **Privacy-first.** No telemetry, no accounts, no server.

## Why this exists

`Invalid signature` is the least useful error message in the EVM. The signature looks fine, the
address looks fine, and `permit()` still reverts — with no way to tell whether the problem is
the chainId, the field order, the `v` byte, or the contract computing its domain separator
differently than your library does.

This happens to real projects. [Permit2 omits `version` from its EIP712Domain while
OpenZeppelin's EIP712 includes it](https://forum.openzeppelin.com/t/permit2-typehash-inconsistency/42049),
so the on-chain and off-chain separators silently disagree.
[Uniswap V2 shipped a `PERMIT_TYPEHASH` that didn't match its own `permit()`
signature](https://github.com/Uniswap/v2-core/pull/79).

why712 reads the **real** domain separator from the contract, recomputes the digest locally,
and tells you exactly what differs — and when it can, **what to change to make it work**.

## Example

```
$ why712 check --typed-data permit.json --signature 0x… --contract 0xA0b8… --signer 0x…

✗ MISMATCH — local digest ≠ on-chain digest

  domain.chainId       expected  1         got  137
  domain.version       expected  (absent)  got  "1"
  signature.v          expected  27/28     got  0        → normalize: v + 27

✓ ROOT CAUSE FOUND
  Dropping `version` from the domain reproduces the contract's domain separator,
  and the signature then recovers to 0xf39F…2266. This contract (Permit2) does not
  declare `version` in EIP712Domain — your signing library adds it by default.
```

## Install

```bash
cargo install why712
```

## Docs

| Doc | What's in it |
|---|---|
| [docs/diagnostics.md](docs/diagnostics.md) | Every diagnostic code: what it means, why it happens, how to fix it |
| [docs/architecture.md](docs/architecture.md) | Crate layering, the pure-core boundary, how RPC stays out of it |
| [docs/coding-standards.md](docs/coding-standards.md) | What gets merged and what doesn't |
| [docs/testing.md](docs/testing.md) | Test vectors, fixtures, snapshots, wasm tests |
| [docs/development.md](docs/development.md) | Local setup, building for wasm, running the demo page |
| [docs/release.md](docs/release.md) | Release playbook |
| [SECURITY.md](SECURITY.md) | Threat model and what never leaves your machine |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: one focused change, tests included,
`cargo fmt` and `cargo clippy` clean, and tell us how you verified it.

## For maintainers

Some repository settings cannot live in a file and have to be set in the GitHub UI: branch
protection on `main` (require the `fmt`, `clippy`, and `test` checks and a PR before merge),
Discussions enabled, the `homepage` field pointing at the demo, and the repository topics.

## License

MIT — see [LICENSE](LICENSE).
