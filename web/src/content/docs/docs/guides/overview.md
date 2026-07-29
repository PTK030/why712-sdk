---
title: Overview
description: What why712 diagnoses and how its CLI and browser surfaces share one Rust core.
---

why712 answers one question: **why does this EIP-712 signature not verify?**

It parses typed data, exposes every EIP-712 intermediate, recovers the signer, and tests a
bounded set of common mistakes. When chain evidence is requested, it compares the locally
computed result with the contract through ERC-5267, `DOMAIN_SEPARATOR()`, and ERC-1271.

## Choose where to start

- [Open the browser playground](/why712/app/) for a quick diagnosis without installation.
- [Run the first example](/why712/docs/guides/getting-started/) to learn the complete workflow.
- [Use the CLI](/why712/docs/guides/cli/) for terminals, scripts, CI, and private RPC endpoints.
- [Read a report](/why712/docs/concepts/report/) to understand verdicts, findings, and remediation.

Both interfaces use the same Rust core. There is no project backend. Given the same request and
the same optional chain snapshot, diagnosis produces the same report.

## What a report contains

A report starts with a verdict and the differences that matter. When a supported change makes
the signature recover the expected signer, why712 includes the exact remediation.

The complete computation remains available:

- `encodeType`
- `typeHash`
- `encodeData`
- `structHash`
- `domainSeparator`
- final signing `digest`

Invalid input is an error. A well-formed signature that does not verify is a finding—the
diagnostic result, not an exceptional state.

## Local by default

The browser runs diagnosis through WebAssembly. Offline mode makes no network request. Optional
contract evidence goes directly to the RPC endpoint you selected.

Share links store inputs in the URL fragment. Review their contents before sending them to
another person.

## Deliberate scope

why712 does not sign messages, accept private keys, submit transactions, operate a wallet, or
support non-EVM chains. See the complete [limits](/why712/docs/reference/limits/).
