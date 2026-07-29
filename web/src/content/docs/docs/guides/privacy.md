---
title: Privacy and security
description: The why712 browser and CLI trust boundary.
---

## Offline by default

The core diagnosis path performs no network, filesystem, clock, or randomness operations.
Browser inputs stay in the current tab unless you explicitly provide both a contract and an RPC
endpoint.

Share links store input in the URL fragment. Browsers do not send fragments in HTTP requests,
but anyone receiving the link can read its contents. Review typed data before sharing.

## Optional network boundary

With RPC enabled, the CLI or browser sends standard JSON-RPC calls only to the endpoint you
selected. Endpoint operators may observe the requested addresses, calldata, IP address, and
timing.

## Sensitive material

Never provide:

- private keys;
- seed phrases;
- wallet backups;
- RPC administrator credentials.

Typed data and signatures can still contain sensitive application or identity information.
Review reports before posting them publicly.

See the repository [security policy](https://github.com/PTK030/why712/blob/main/SECURITY.md) for
the complete threat model and vulnerability reporting process.
