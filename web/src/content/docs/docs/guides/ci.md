---
title: Use why712 in CI
description: Run deterministic EIP-712 checks in scripts and continuous integration.
---

Use the CLI's JSON output and exit codes when a signature check is part of an automated
workflow.

```bash
why712 check \
  --typed-data permit.json \
  --signature "$SIGNATURE" \
  --signer "$EXPECTED_SIGNER" \
  --format json \
  --offline
```

## Exit codes

- `0` means the supplied evidence matches.
- `1` means why712 produced a mismatch or inconclusive report.
- `2` means the command could not diagnose the request because input, usage, or setup failed.

Treat code `1` as a diagnostic result, not an application crash. Parse the stable finding
`code` values in JSON rather than matching human-readable messages.

## Keep the job deterministic

Use `--offline` when CI should not depend on an RPC endpoint. Provide all input files through the
job workspace and pin the installed why712 version.

If contract evidence is required, provide both `--contract` and `--rpc`. RPC failures are
reported and offline diagnosis continues, but network evidence will be incomplete.
