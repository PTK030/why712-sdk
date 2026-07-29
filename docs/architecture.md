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
failure. The report emits `SIGNER_IS_CONTRACT` and remains inconclusive until full ERC-1271
validation is available.

## Verified sources

Checked 2026-07-29:

- [ERC-5267](https://eips.ethereum.org/EIPS/eip-5267) defines the exact ABI, LSB-first bitmap,
  extension semantics, and requires returned values to describe the separator used for
  verification.
- [ERC-1271](https://eips.ethereum.org/EIPS/eip-1271) defines
  `isValidSignature(bytes32,bytes)` and the `0x1626ba7e` success value.
- The pinned local source for Alloy 2.2.0 confirms `provider-http` plus
  `reqwest-rustls-tls` is the native rustls feature set. The adapter uses only public
  `RootProvider` and `Provider` APIs.
