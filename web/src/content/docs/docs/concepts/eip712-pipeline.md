---
title: The EIP-712 pipeline
description: The values why712 computes between typed data and the final signing digest.
---

EIP-712 verification is a sequence of related computations. A signature can be mathematically
valid while still recovering the wrong signer because one earlier value differs.

## 1. Type encoding

`encodeType` is the canonical text representation of the primary type and its dependencies.
Its Keccak-256 hash is `typeHash`.

Field order and integer widths are part of this representation. `uint160` and `uint256` do not
produce the same type hash.

## 2. Struct encoding

`encodeData` combines the type hash with each encoded field. Dynamic `string` and `bytes`
values are hashed before they enter the struct encoding.

Hashing the encoded data produces `structHash`.

## 3. Domain separator

The EIP-712 domain identifies the signing context. Common fields include the application name,
version, chain ID, and verifying contract.

The domain is encoded like another struct and hashed into `domainSeparator`.

## 4. Signing digest

The final digest is:

```text
keccak256(0x1901 || domainSeparator || structHash)
```

Signature recovery uses this digest. why712 exposes every intermediate so you can identify
where two implementations stop agreeing.

## Contract evidence

When RPC is enabled, why712 can compare the local domain with `eip712Domain()` and
`DOMAIN_SEPARATOR()` from the verifying contract. This evidence enriches the report; it does
not change the deterministic computation.
