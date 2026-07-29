---
title: Finding codes
description: How to use why712's stable diagnostic codes and authoritative catalog.
---

Each finding has a stable `SCREAMING_SNAKE_CASE` code, severity, confidence, human explanation,
and remediation. Scripts should branch on the code rather than message text.

Common groups include:

- `DOMAIN_*` — domain fields or separator evidence disagree;
- `SIGNATURE_*` — recovery encoding, scalar, signer, or ERC-1271 behavior;
- `TYPE_*` and `FIELD_*` — primary-type definitions or field ordering differ;
- `ENCODING_*` — packed encoding, unhashed dynamic values, or envelope mistakes;
- `ONCHAIN_*` — the source and quality of contract evidence.

Use the CLI to explain a code:

```bash
why712 explain DOMAIN_CHAIN_ID_MISMATCH
```

The authoritative catalog is versioned with the core in
[`docs/diagnostics.md`](https://github.com/PTK030/why712/blob/main/docs/diagnostics.md). CI fails
when a Rust `FindingCode` is missing from that catalog, duplicated, or serialized incorrectly.
