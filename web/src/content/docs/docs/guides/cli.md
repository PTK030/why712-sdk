---
title: CLI
description: why712 commands, input sources, output formats, networking, and exit codes.
---

## Commands

| Command | Purpose |
|---|---|
| `why712 check` | Diagnose a signature, optionally enriched through RPC |
| `why712 digest` | Print canonical EIP-712 intermediate hashes |
| `why712 examples` | List bundled reproducible cases |
| `why712 explain` | Explain a stable finding code |

Typed data may come from a file or standard input with `--typed-data -`. An unmodified wallet
request may instead use `--wallet-request`; why712 extracts `params[1]` only when the method is
exactly `eth_signTypedData_v4`.

```bash
why712 check \
  --typed-data permit.json \
  --signature 0x... \
  --signer 0x... \
  --format json \
  --offline
```

Output formats are text, versioned JSON, and GitHub-flavored Markdown.

## RPC mode

A live probe requires both `--contract` and `--rpc`. `WHY712_RPC_URL` may provide the endpoint,
but `--offline` always prevents network access even when the environment variable is set.

```bash
why712 check \
  --typed-data permit.json \
  --signature 0x... \
  --signer 0x... \
  --contract 0x... \
  --rpc https://your-endpoint.example
```

For recognized EIP-2612 and Permit2 inputs, the live probe also compares nonce and deadlines
with state from the observed block. EIP-2612 is simulated with a read-only `eth_call`.

Counterfactual signatures are detected automatically. To validate one, pass the expected
signer, RPC, contract, and a reviewed compatible deployment with
`--erc6492-validator 0x...`. The call is bounded and read-only.

RPC enrichment failure is printed to standard error and deterministic offline diagnosis
continues.

## Exit codes

- `0`: supplied evidence matches.
- `1`: diagnosed non-match or inconclusive result.
- `2`: usage, malformed input, filesystem, or RPC setup failure.
