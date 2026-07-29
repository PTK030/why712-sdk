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

## Shared example corpus

`crates/why712-core/fixtures/examples/` is the single source for core integration tests,
`why712 examples`, and the future web demo:

- each named `.json` file is a plain EIP-712 typed-data document suitable for passing directly
  to `why712 --typed-data`;
- `manifest.json` supplies presentation metadata, the optional signature and signer, and a
  machine-readable fault name;
- `fault: null` denotes a valid control case;
- signature and signer are either both present or both absent.

`tests/fixtures.rs` rejects duplicate IDs, empty metadata, path traversal, malformed signatures
or addresses, missing files, invalid typed data, and examples that cannot be hashed. It also
proves that the initial wrong-chain-ID and wrong-field-order cases differ from their valid
control digest.

## Required checks

Run these after every milestone:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p why712-core --target wasm32-unknown-unknown
```
