---
title: How to read a report
description: Understand verdicts, findings, remediation, and the complete EIP-712 computation.
---

A why712 report is ordered from the decision you need to the technical detail behind it.

## Verdict

- `MATCH` means the supplied evidence agrees.
- `MISMATCH` means the supplied inputs do not verify.
- `INCONCLUSIVE` means more evidence is needed for a definitive answer.

Malformed input is an error instead of a verdict because why712 could not perform diagnosis.

## Findings

Each finding includes a stable code, severity, confidence, message, and optional expected and
actual values. When why712 can test a modified digest and recover the expected signer,
confidence is `CERTAIN`.

Use the finding code in scripts and links. Human-readable text may improve between releases.

## Remediation

When a supported change makes the signature recover the intended signer, the finding includes a
specific remediation. Apply the change at the point where typed data or the signature is
created, then sign again.

## Evidence and computation

Signer recovery shows expected and recovered addresses. Input and contract state records the
source and optional chain snapshot. The computation section contains `encodeType`, `typeHash`,
`encodeData`, `structHash`, `domainSeparator`, and the final digest.
