---
title: Use contract evidence
description: How why712 reads EIP-712 domains and validates contract signers.
---

RPC is enrichment, not part of the diagnosis engine. The CLI or browser first acquires a plain
snapshot and then gives that value to the same pure core used offline.

Provide a verifying contract together with an RPC endpoint. Without both values, why712 stays
on the offline path.

## Domain evidence

why712 probes `eip712Domain()` as defined by ERC-5267 and independently calls
`DOMAIN_SEPARATOR()`. The second value detects contracts whose structured metadata disagrees
with the separator actually used for verification.

If ERC-5267 is unavailable, the opaque separator remains useful evidence. If both optional
calls fail, their failures are retained in the report and offline diagnosis continues.

## Permit state

Recognized EIP-2612 messages are compared with `nonces(owner)` and the observed block timestamp,
then the permit call is simulated without broadcasting a transaction. Permit2 ordered
allowances use `allowance(owner, token, spender)`; unordered transfers use `nonceBitmap`.
Incomplete execution calldata is reported as inconclusive rather than guessed.

## Contract signers

When the expected signer has runtime code, why712 does not treat failed ECDSA recovery as proof
that the signature is invalid. It calls ERC-1271
`isValidSignature(bytes32,bytes)` with the canonical digest and complete opaque signature.

- `0x1626ba7e` means match.
- Another ABI-valid `bytes4` means mismatch.
- A revert, transport failure, or malformed response is inconclusive.

ERC-6492 wrappers are detected before ERC-1271 or ECDSA. Detection alone is inconclusive until
a compatible validator supplied by the caller provides explicit evidence. why712 limits the
wrapper to 128 KiB and the read-only call to five million gas; it does not hard-code a third
party validator or broadcast factory calldata.

## Browser RPC

The playground contacts the endpoint directly from the browser. The endpoint must allow CORS.
why712 never routes the request through a project proxy.
