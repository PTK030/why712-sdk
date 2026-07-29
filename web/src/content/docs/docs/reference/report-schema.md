---
title: Report schema
description: Stable structure and compatibility rules for why712 JSON reports.
---

JSON output is versioned. Consumers should check `schemaVersion`, branch on stable enum and
finding-code values, and ignore unknown fields so additive releases remain compatible.

## Top-level fields

| Field | Meaning |
|---|---|
| `schemaVersion` | Version of the serialized report contract |
| `verdict` | `MATCH`, `MISMATCH`, or `INCONCLUSIVE` |
| `findings` | Ordered diagnostic findings |
| `expectedSigner` | Expected address when supplied |
| `recoveredSigner` | Address recovered from the canonical digest |
| `context` | Input source, detected standard, block, and optional protocol snapshot |
| `computed` | EIP-712 intermediate values |

## Finding fields

Every finding has `code`, `severity`, `confidence`, and `message`. It may also contain
`expected`, `actual`, and `remediation`.

Scripts should use `code`, not `message`, as the stable identifier.

## Browser representation

The WebAssembly bindings expose camelCase JavaScript objects. The playground passes the same
report object to the shared Markdown renderer; it does not reconstruct diagnosis in JavaScript.

Schema version 2 adds `context` and protocol snapshots. Consumers of version 1 must branch on
`schemaVersion`; current readers accept a missing optional context when rendering older reports.
