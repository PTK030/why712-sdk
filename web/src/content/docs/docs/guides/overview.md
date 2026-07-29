---
title: Overview
description: What why712 diagnoses and how its CLI and browser surfaces share one Rust core.
---

why712 answers one question: **why does this EIP-712 signature not verify?**

It parses typed data, exposes every EIP-712 intermediate, recovers the signer, and tests a
bounded set of common mistakes. When chain evidence is requested, it compares the locally
computed result with the contract through ERC-5267, `DOMAIN_SEPARATOR()`, and ERC-1271.

## Two interfaces, one engine

- The native `why712` CLI is designed for terminals, scripts, CI, and private RPC endpoints.
- The [browser playground](/why712/app/) runs the same deterministic Rust diagnosis engine
  through WebAssembly.

There is no project backend. Diagnosis is synchronous and deterministic: the same request and
the same optional chain snapshot produce the same report.

## What a report contains

A report has a verdict, stable finding codes, actionable remediation, expected and actual
values, recovered signer evidence, and the complete EIP-712 computation:

- `encodeType`
- `typeHash`
- `encodeData`
- `structHash`
- `domainSeparator`
- final signing `digest`

Invalid input is an error. A well-formed signature that does not verify is a finding — that is
the product, not an exceptional state.

## Deliberate limits

why712 does not sign messages, accept private keys, submit transactions, operate a wallet, or
support non-EVM chains.
