---
title: CLI reference
description: why712 commands, input sources, output formats, networking, and exit codes.
---

## Commands

| Command | Purpose |
|---|---|
| `why712 check` | Diagnose a signature, optionally enriched through RPC |
| `why712 digest` | Print canonical EIP-712 intermediate hashes |
| `why712 examples` | List bundled reproducible cases |
| `why712 explain` | Explain a stable finding code |

Typed data may come from a file or standard input with `--typed-data -`.

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

RPC enrichment failure is printed to standard error and deterministic offline diagnosis
continues.

## Exit codes

- `0`: supplied evidence matches.
- `1`: diagnosed non-match or inconclusive result.
- `2`: usage, malformed input, filesystem, or RPC setup failure.
