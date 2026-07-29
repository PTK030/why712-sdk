# Architecture

why712 has one deterministic diagnosis engine and two delivery surfaces: a native CLI and a
statically generated Astro website. The site contains an Astro landing page, Starlight user
documentation, and a vanilla TypeScript playground using the WASM bindings. There is no React
runtime and no backend.

## Dependency direction

`why712-core` owns typed-data parsing, Alloy-backed EIP-712 computation, signature recovery,
hypothesis search, chain snapshot value types, findings, and report rendering. The CLI and WASM
crates may depend on the core; the core never depends on either front end.

Diagnosis itself is pure and synchronous. It receives a `DiagnoseRequest`, including an optional
plain `OnchainSnapshot`, and produces the same report for the same values. It does not read a
file, contact a chain, inspect the clock, or use randomness.

Chain acquisition is deliberately separate:

1. `ChainClient` is the minimal asynchronous port: `eth_call`, `eth_chainId`, `eth_getCode`,
   and block number/timestamp lookup.
2. `acquire_onchain_snapshot` probes the chain and returns a plain value.
3. The caller passes that value to `diagnose_offline`.

The native Alloy adapter is compiled only with `why712-core/rpc`. The default core graph, used by
the browser build, contains no provider or Tokio transport. The WASM crate implements the same
port over `window.fetch`.

Astro is a build-time delivery layer only. It never owns hashing, diagnosis, report rendering,
or RPC semantics. `scripts/build-web.mjs` first builds the Rust bindings and copies the shared
fixture corpus into the public playground tree, then Astro emits the complete static artifact
to `web/dist`.

## Stateful signature protocols

Typed data is recognized only when its primary type and required fields exactly match EIP-2612
or an implemented Permit2 shape. RPC acquisition records nonce, deadline, block timestamp, and
read-only call results in a plain `ProtocolSnapshot`; diagnosis never reads chain state or the
local clock. Protocol reads and simulations use the captured block number rather than a later
`latest` state.

EIP-2612 acquisition reads `nonces(owner)` and can simulate the exact `permit` call. Permit2
allowances read `allowance(owner, token, spender)`; SignatureTransfer reads `nonceBitmap`.
Permit2 execution is not synthesized without execution calldata because its effective spender
and transfer details are call-context dependent.

An ERC-6492 suffix is recognized before ERC-1271 and ECDSA. Wrapper detection alone remains
inconclusive until a caller-selected compatible validator supplies explicit evidence. The
validator call rejects wrappers above 128 KiB and carries a five-million-gas ceiling; why712
does not hard-code an external deployment or broadcast the factory call.

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
- [ERC-2612](https://eips.ethereum.org/EIPS/eip-2612) defines `nonces(owner)`, deadline checks,
  the canonical permit fields, and the requirement that invalid permits revert.
- [ERC-6492](https://eips.ethereum.org/EIPS/eip-6492) defines the detection suffix and requires
  wrapper validation before ERC-1271 or ECDSA.
- [Uniswap Permit2 interfaces](https://github.com/Uniswap/permit2/tree/main/src/interfaces)
  define the ordered allowance tuple and unordered nonce bitmap used by protocol snapshots.
- The pinned local source for Alloy 2.2.0 confirms `provider-http` plus
  `reqwest-rustls-tls` is the native rustls feature set. The adapter uses only public
  `RootProvider` and `Provider` APIs.
