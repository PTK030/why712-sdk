# Testing

why712 tests hashing against externally produced vectors. A digest copied from the implementation
under test would only confirm that the implementation repeats itself.

## Canonical EIP-712 Mail vector

`crates/why712-core/fixtures/vectors/eip712-mail.json` is the `Mail` request from the
[EIP-712 specification](https://eips.ethereum.org/EIPS/eip-712). The specification defines the
encoding and publishes a signature for that exact request.

The expected domain separator
`0xf2cee375fa42b42143804025fc449deafd50cc031ca257e0b194a650a912090f` and signing digest
`0xbe609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2`
are cross-checked against the independent
[`ethers_eip712` test](https://docs.rs/ethers_eip712/latest/src/ethers_eip712/lib.rs.html#135-203).
Sources last checked on 2026-07-29.

## Arrays and dynamic bytes

`crates/why712-core/fixtures/vectors/arrays-and-bytes.json` combines nested structs, dynamic
struct arrays, a dynamic integer array, `string`, non-empty `bytes`, and empty `bytes`.

The expected values were generated independently with
[`hashTypedData`](https://viem.sh/docs/utilities/hashTypedData), `hashStruct`, and `hashDomain`
from Viem **2.45.0**, installed only in an isolated temporary directory:

| Value | Expected |
|---|---|
| `typeHash` | `0x02aefee3ee5461543f44be7c8844e025bab8fad50cf88734cc6249cb6bf30622` |
| `domainSeparator` | `0x65c0a9b48dbff1f16f875f2ca382668feda1d9cff27bf1eec52b46232dc12df1` |
| `hashStruct` | `0x2149540e43ac5474d5e5d447d64b97800bb3dbd6ccefbe63e1c9365932ca8e43` |
| signing digest | `0xc669c5801d3e3fb634fc85343807d7432d334886fcfc18bac2da899169a71bec` |

Source and package version last checked on 2026-07-29. Viem is not a project dependency.

## Numeric JSON values

`serde_json` is built with `arbitrary_precision`. A raw integer literal such as
`9007199254740993` (one above JavaScript's safe-integer limit) survives Rust parsing exactly and
produces the same digest as the string `"9007199254740993"`.

Pinned `alloy-dyn-abi` 1.6.1 accepts numeric `uint` values through `Value::as_u64()`. Therefore:

- raw integer literals up to `u64::MAX` are accepted exactly by the Rust path;
- larger `uint256` values must be JSON strings;
- `MESSAGE_NUMERIC_PRECISION_LOSS` must inspect the original JSON representation and warn on
  numeric integer fields above JavaScript's safe range, because a JS caller may have rounded the
  number before serialization even though Rust itself does not.

These boundaries are covered by `tests/vectors.rs`. Dependency source and behavior last checked
on 2026-07-29.

## Signature parsing and recovery

`tests/signature.rs` uses the signature published with the canonical EIP-712 `Mail` request to
verify address recovery. It also derives equivalent encodings from that independently published
signature and covers:

- canonical 65-byte `r || s || v` signatures with raw `v` (`0`/`1`), historical `v`
  (`27`/`28`), and transaction-style EIP-155 values;
- compact 64-byte ERC-2098 signatures, where the parity bit occupies the top bit of canonical
  low-S;
- a mathematically equivalent high-S twin, which remains recoverable but is retained as a
  diagnosable non-canonical state;
- zero scalar values, invalid recovery bytes, malformed hex, and unsupported lengths.

The high-S behavior follows [EIP-2](https://eips.ethereum.org/EIPS/eip-2): Homestead rejects
high-S transaction signatures, while the `ecrecover` precompile remains able to recover them.
The compact representation follows [ERC-2098](https://eips.ethereum.org/EIPS/eip-2098).
Specifications last checked on 2026-07-29.

## Shared example corpus

`crates/why712-core/fixtures/examples/` is the single source for core integration tests,
`why712 examples`, and the future web demo:

- each named `.json` file is a plain EIP-712 typed-data document suitable for passing directly
  to `why712 --typed-data`;
- `manifest.json` supplies presentation metadata, the optional signature and signer, and a
  machine-readable fault name;
- every signed entry records `signedDigest`, so tests prove both that the signature belongs to
  the expected signer and, for a broken typed-data document, that its canonical digest does not
  verify that signature;
- `fault: null` denotes a valid control case;
- signature, signer, and signed digest are either all present or all absent.

`tests/fixtures.rs` rejects duplicate IDs, empty metadata, path traversal, malformed signatures
or addresses, missing files, invalid typed data, and examples that cannot be hashed. It proves
that wrong-chain-ID and wrong-field-order cases differ from their valid control digest, and
checks signer recovery plus signature classification for canonical, raw-`v`, high-S, and
ERC-2098 examples.

The corpus's independently generated cases use the public Anvil development key
`0xac0974…ff80` (address `0xf39F…2266`) and pinned Viem 2.45.0. They cover:

- `abi.encodePacked` applied to a primary struct instead of EIP-712's 32-byte field encoding;
- the Permit2 domain with an unexpected `version`;
- a domain whose signed form contains a `bytes32 salt` omitted by the supplied document;
- Permit2 detail fields widened from `uint160`/`uint48` to `uint256`.

For the packed case, Viem independently produced:

| Value | Expected |
|---|---|
| domain separator | `0x2e667b98e0fd80dec52c40795f5f78feff691534aab0743c9639568d4bea26af` |
| `Payment` type hash | `0x477aa8c025938ae0e68308ba65cb20d25ab38ce00aeb930ff8fec11ab5f3e63a` |
| packed struct hash | `0x252753f9591304025d6e755c74f44397277f11ef131559372f73a8875424c911` |
| signed digest | `0x71d31fd1a44c6681123136d7726e4f10baf874fc9d2f5ee77d41c29885c633d3` |

Fixture signatures and hashes last regenerated and checked on 2026-07-29. Viem and the
development private key are test-vector tooling only; neither is a project dependency or a
production secret.

## Hypothesis engine

`tests/hypothesis.rs` proves that candidate generation is deterministic, sorted by rank, and
hard-limited to 512 candidates even when an eight-field type has more permutations. Search
tests require a mutation to recover the expected signer before it becomes a hit.

Canonical document mutations are always parsed and hashed again by `alloy-dyn-abi`. Alternative
encoding strategies also delegate their ABI work to pinned alloy:

- `DynSolValue::abi_encode_packed()` for packed struct members;
- `DynSolValue::abi_encode_params()` for the intentionally incorrect direct encoding of
  dynamic `string`/`bytes` values;
- `eip191_hash_message()` for `personal_sign`.

`abi_encode()` is not interchangeable with `abi_encode_params()` for a dynamic tuple: the
former treats the tuple as one argument and emits an outer offset, while the latter encodes its
members as the parameter list. This distinction is covered by the independently generated
unhashed-dynamic vector.

Pinned Viem 2.45.0 and the public Anvil development key independently produced these alternate
digests and signatures:

| Strategy | Signed digest |
|---|---|
| packed `Payment` members | `0x71d31fd1a44c6681123136d7726e4f10baf874fc9d2f5ee77d41c29885c633d3` |
| bare `Payment` struct hash | `0xa21e55ea73f30e2f44684281052d13f505d1876037b8d257b349cec861046aff` |
| domain + struct hash without `0x1901` | `0xceebda57cff755fd6daebd3caa923cb18eb35e142d570e3c08892804a5d24b37` |
| EIP-191 over the exact `encode-packed.json` text | `0xdba6b87febac084370df9d377623ddbfd135d3304680042ac90b2d3e117e47c7` |
| unhashed dynamic `Note` values | `0x9c08459f5120ac358431a23027e0da0bf16c71f1f9ce0603ae30543063a473ef` |

Sources and vectors last checked on 2026-07-29.

## Offline diagnosis reports

`tests/integration_test.rs` runs `diagnose_offline()` across every entry in the shared manifest.
Each example asserts its verdict and defining `FindingCode`, then snapshots the complete
versioned JSON report under `tests/snapshots/`.

The integration contract covers:

- `MATCH` for canonical and ERC-2098 signatures;
- direct signature findings for raw `v` and high-S;
- `CERTAIN` root causes only when a candidate digest recovers the expected signer;
- an accompanying canonical `SIGNER_MISMATCH` showing the actual recovered address;
- `INCONCLUSIVE` plus a heuristic precision warning when no signature or signer is supplied.

`tests/diagnose.rs` separately proves zero scalars, invalid recovery bytes, digest-only use, and
the boundary where malformed signatures remain `Error` rather than findings. Snapshot files are
LF-normalized by `.gitattributes`; they pass under the native Windows toolchain.

## Markdown reports

`tests/markdown.rs` snapshots both a matching report and a proven root-cause report. The
renderer consumes only `Report`; it never re-hashes input or repeats diagnosis logic. A
separate assertion covers escaping pipes, backslashes, and line breaks that would otherwise
break GitHub-flavored Markdown tables.

The output deliberately uses ASCII status labels and separators so the same report remains
readable in Windows terminals, redirected files, GitHub issues, and browser clipboard output.

## Chain snapshots and RPC

`tests/chain.rs` uses a fake `ChainClient` with no HTTP. It proves the ERC-5267 failure path
falls back to `DOMAIN_SEPARATOR()`, retains the failed probe as evidence, records chain and code
observations, and detects a contract signer. Unit tests beside `eip712/domain.rs` cover the
LSB-first bitmap, canonical separator reconstruction, and refusal to guess extension semantics.

`tests/rpc.rs` is enabled only by the `rpc` feature. A local wiremock server verifies the base
`eth_chainId`, `eth_getCode`, and `eth_call` mappings. Block metadata and gas-limited call
behavior are covered by transport and protocol tests without depending on a public endpoint.

Direct diagnosis tests prove on-chain separator mismatch, `SIGNER_IS_CONTRACT`,
`ONCHAIN_DOMAIN_SOURCE`, and all three ERC-1271 outcomes. Transport tests pass a non-65-byte
opaque signature to the signer contract and verify that the standard magic value yields a match,
a non-magic `bytes4` yields a mismatch, and a revert or malformed return stays inconclusive.

`tests/protocol.rs` covers wallet-request normalization, EIP-2612 nonce mismatch, millisecond
deadlines, read-only permit reverts, Permit2 bitmap reuse, and ERC-6492 detection before ECDSA
length parsing. Protocol fixtures inject plain snapshots so expected results never depend on a
live endpoint or wall clock. A missing nonce or historical state produces
`PROTOCOL_STATE_UNAVAILABLE` and an inconclusive verdict. An unavailable protocol simulation
also overrides an otherwise matching local signature to `INCONCLUSIVE`, preventing an RPC or
archive-node limitation from becoming a false success.

`fixtures/protocol/eip2612-stale-nonce.json` uses the exact ERC-2612 type from the specification
and records the public ethers.js discussion that motivated the nonce/deadline regression. Its
chain observations are explicit fixture values, not explorer output or values generated by
why712.

## CLI

`crates/why712-cli/tests/cli.rs` executes the compiled binary. It covers:

- text, versioned JSON, and Markdown report formats;
- `check`, `digest`, `examples`, and `explain`;
- exit 0 for a match, 1 for a non-match, and 2 for usage/input/I/O failures;
- explicit RPC failure on stderr followed by offline fallback;
- `--offline` overriding `WHY712_RPC_URL`;
- stable digest and finding-code output from shared fixtures.

## Container smoke tests

The production Docker build must complete from the repository root, then run with
`--network none --read-only` against the canonical Mail fixture. The image must report `MATCH`,
run as the distroless non-root user, and expose only `/usr/local/bin/why712` as its entrypoint.

Validate `docker-compose.yml` with a non-secret `MAINNET_RPC_URL`, start the Anvil healthcheck,
and confirm chain ID 1 plus real `DOMAIN_SEPARATOR()` responses from mainnet USDC and Permit2.
The fork is ephemeral; remove its container and network with `docker compose down`.

## WASM browser contract

`crates/why712-wasm/tests/web.rs` runs through `wasm-bindgen-test` in headless Chrome. It proves:

- the canonical Mail fixture returns the same `MATCH` verdict as the native integration test;
- synchronous and asynchronous no-RPC bindings agree;
- reports and the shared example manifest cross the boundary as plain JavaScript objects, not
  ES2015 `Map` values;
- generated report fields use the documented camelCase shape;
- the example and Markdown helper bindings return usable JavaScript values;
- the exact current report round-trips through JavaScript and into the shared Markdown renderer.

The crate also builds its own browser `fetch` transport for `eth_call`, `eth_chainId`, and
`eth_getCode`; a CORS or JSON-RPC failure becomes `ONCHAIN_DOMAIN_SOURCE` evidence and does not
discard the offline report. Generated `pkg/` files are build artifacts and remain ignored.

See `docs/development.md` for the exact `wasm-pack` commands and the Windows ChromeDriver
version-selection caveat.

`web/src/scripts/share.test.mjs` separately proves that fragment-only links round-trip Unicode input,
remain URL-safe, and discard fields outside the public request shape. The static build copies
only files referenced by the core fixture manifest; the browser never maintains a second
example corpus.

## Required checks

Run these after every milestone:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```
