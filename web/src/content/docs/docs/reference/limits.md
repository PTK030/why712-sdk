---
title: Limits
description: What why712 deliberately does not do.
---

why712 is deliberately narrow. It diagnoses EIP-712 verification; it is not a wallet or signing
service.

## Out of scope

why712 does not:

- accept or manage private keys;
- sign typed data;
- submit transactions;
- connect to a wallet;
- store reports on a backend;
- support non-EVM signing formats;
- infer arbitrary proprietary contract behavior;
- guarantee that a semantically dangerous message is safe to sign.

## Input limits

Raw JSON integer literals are exact through `u64::MAX` in the Rust path. Larger `uint256` values
must be represented as strings. Values already rounded by JavaScript cannot be recovered.

The hypothesis search is deterministic and bounded. An inconclusive report can therefore mean
that the actual error is outside the supported candidate set.

## RPC limits

Browser endpoints must allow CORS. Contracts may omit ERC-5267 or implement custom verification
logic. Failed optional probes remain visible as evidence and do not prevent offline diagnosis.

ERC-6492 validation requires a caller-selected compatible validator. Wrappers are limited to
128 KiB and the read-only call to five million gas.

Transaction-hash-only diagnosis is not exposed yet: mined permits require parent-block state,
and nested router calldata does not necessarily contain the original typed-data schema.
