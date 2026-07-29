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

## Required checks

Run these after every milestone:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```
