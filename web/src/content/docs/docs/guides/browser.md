---
title: Browser playground
description: Diagnose a signature locally in the browser without installing why712.
---

The browser playground runs the same Rust diagnosis engine as the CLI through WebAssembly. It
does not require an account, a wallet connection, or a why712 backend.

## Run a bundled example

1. Open the [playground](/why712/app/).
2. Choose an item under **Load an example**.
3. Select **Diagnose signature**.
4. Read **What differs** before expanding the complete computation.

Bundled examples come from the same fixture corpus used by the Rust integration tests.

## Diagnose your own request

Choose the input format that matches what you captured:

- **Typed data** accepts the EIP-712 object with `types`, `primaryType`, `domain`, and `message`.
- **Wallet request** accepts the complete `eth_signTypedData_v4` JSON-RPC request.

Add the public signature and, when known, the expected signer. The expected signer lets why712
test whether a specific change produces the signer you intended.

## Stay offline

Clear the RPC endpoint before running the diagnosis. Without both an RPC endpoint and verifying
contract, the browser uses the synchronous offline path and makes no network request.

## Share a result

**Copy as Markdown** creates a report suitable for an issue or pull request. **Copy link** stores
the current inputs in the URL fragment. The fragment is not sent in HTTP requests, but anyone
with the link can read it, so review typed data before sharing.

Never provide a private key, seed phrase, wallet backup, or privileged RPC credential.
