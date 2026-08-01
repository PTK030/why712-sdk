# Architecture

why712 has one deterministic diagnosis engine and three embeddable delivery surfaces: the
`why712-core` Rust crate, the `why712` TypeScript package, and the native `why712` CLI.
There is no hosted application or backend.

## Dependency direction

`why712-core` owns typed-data parsing, Alloy-backed EIP-712 computation, signature recovery,
hypothesis search, chain snapshot value types, findings, and report rendering. The CLI and WASM
crates may depend on the core; the core never depends on either adapter. The TypeScript facade
depends only on generated WASM bindings and contains no duplicate signature logic.

Diagnosis itself is pure and synchronous. It receives a `DiagnoseRequest`, including an optional
plain `OnchainSnapshot`, and produces the same report for the same values. It does not read a
file, contact a chain, inspect the clock, or use randomness.

Chain acquisition is deliberately separate:

1. `ChainClient` is the minimal asynchronous port: block-aware `eth_call`/`eth_getCode`,
   `eth_chainId`, transaction lookup, and block number/timestamp lookup.
2. `acquire_onchain_snapshot` probes the chain and returns a plain value.
3. The caller passes that value to `diagnose_offline`.

The native Alloy adapter is compiled only with `why712-core/rpc`. The default core graph, used by
the SDK build, contains no provider or Tokio transport. The WASM crate implements the same port
over `globalThis.fetch`, which is available in modern Node.js and browsers.

`scripts/build-sdk.mjs` compiles the WASM adapter twice. The `web` target is embedded under the
package's browser conditional export; the `nodejs` target is embedded under its Node export.
`packages/why712-sdk/src/{browser,node}.ts` hide loader differences and expose the same camelCase,
Promise-based API. The npm tarball is tested after extraction so ignored or missing WASM files
cannot pass merely because an unpackaged working tree happened to contain them.

```text
application or test
        |
        v
why712 TypeScript facade
        |
        v
why712-wasm (browser or Node loader)
        |
        v
why712-core (pure deterministic diagnosis)
```

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

Transaction-hash diagnosis accepts only exact direct-call selectors from ERC-2612 and the
Uniswap Permit2 interfaces. A mined transaction is reconstructed and simulated against its
parent block; a pending transaction uses the endpoint's current `latest` block. The original
sender is retained because Permit2 hashes `msg.sender` as `spender` for SignatureTransfer.
Unknown selectors, routers, witness permits, missing ERC-5267 metadata, and archive-state
refusals are never heuristically decoded. Nested and witness flows require the original wallet
request.

An ERC-6492 suffix is recognized before ERC-1271 and ECDSA. The canonical
`(address,bytes,bytes)` wrapper is decoded to expose the factory/prepare target and byte lengths
without executing it. Wrapper detection alone remains
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
- [Uniswap PermitHash](https://github.com/Uniswap/permit2/blob/main/src/libraries/PermitHash.sol)
  defines the exact SignatureTransfer type strings and confirms that `msg.sender` is the signed
  `spender`.
- The pinned local source for Alloy 2.2.0 confirms `provider-http` plus
  `reqwest-rustls-tls` is the native rustls feature set. The adapter uses only public
  `RootProvider` and `Provider` APIs.
