# Architecture

why712 has one deterministic diagnosis engine and two delivery surfaces: a native CLI and a
browser demo. There is no backend.

## Dependency direction

`why712-core` owns typed-data parsing, Alloy-backed EIP-712 computation, signature recovery,
hypothesis search, chain snapshot value types, findings, and report rendering. The CLI and WASM
crates may depend on the core; the core never depends on either front end.

Diagnosis itself is pure and synchronous. It receives a `DiagnoseRequest`, including an optional
plain `OnchainSnapshot`, and produces the same report for the same values. It does not read a
file, contact a chain, inspect the clock, or use randomness.

Chain acquisition is deliberately separate:

1. `ChainClient` is the minimal asynchronous port: `eth_call`, `eth_chainId`, and `eth_getCode`.
2. `acquire_onchain_snapshot` probes the chain and returns a plain value.
3. The caller passes that value to `diagnose_offline`.

The native Alloy adapter is compiled only with `why712-core/rpc`. The default core graph, used by
the browser build, contains no provider or Tokio transport. The WASM crate implements the same
port over `window.fetch`.

## Domain probing

The probe order is `eip712Domain()` and then `DOMAIN_SEPARATOR()`. The second call is made even
after a successful ERC-5267 response because it provides the independent value needed to detect
a self-inconsistent contract. If ERC-5267 fails, the opaque separator is the fallback. If both
optional calls fail, their errors remain in snapshot evidence and diagnosis continues offline.

ERC-5267 defines bits 0 through 4, least-significant first, for `name`, `version`, `chainId`,
`verifyingContract`, and `salt`. Values whose bits are absent are unspecified and must be
discarded. why712 does not guess a separator when the response contains unknown bits or
extensions it does not implement.

When an expected signer has non-empty runtime code, ECDSA recovery is not treated as proof of
failure. If signature input is available, acquisition calls
`isValidSignature(bytes32,bytes)` on the signer with the canonical EIP-712 digest and the full
opaque signature bytes. The pure diagnosis step then treats the standard `0x1626ba7e` return as
a match, another ABI-valid `bytes4` as a mismatch, and a revert, transport failure, or malformed
return as inconclusive. Smart-account signatures are not forced into a 64/65-byte ECDSA shape.

## Verified sources

Checked 2026-07-29:

- [ERC-5267](https://eips.ethereum.org/EIPS/eip-5267) defines the exact ABI, LSB-first bitmap,
  extension semantics, and requires returned values to describe the separator used for
  verification.
- [ERC-1271](https://eips.ethereum.org/EIPS/eip-1271) defines
  `isValidSignature(bytes32,bytes)`, the `0x1626ba7e` success value, a read-only call that may
  invoke arbitrary validation logic, and no fixed gas limit for callers.
- The pinned local source for Alloy 2.2.0 confirms `provider-http` plus
  `reqwest-rustls-tls` is the native rustls feature set. The adapter uses only public
  `RootProvider` and `Provider` APIs.
