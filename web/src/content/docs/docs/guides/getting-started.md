---
title: Getting started
description: Run your first offline EIP-712 diagnosis in the browser, from source, or with Docker.
---

## Browser

Open the [diagnostic playground](/why712/app/), select a reproducible case, and choose
**Diagnose signature**. Clear the RPC field to guarantee an offline run.

Never paste a private key or seed phrase. why712 only needs typed data, a public signature, and
optionally the expected signer.

## Run from source

Rust 1.97 is pinned by the repository:

```bash
cargo run -p why712 -- examples
cargo run -p why712 -- check \
  --typed-data crates/why712-core/fixtures/examples/wrong-chain-id.json \
  --signature 0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c \
  --signer 0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826 \
  --chain-id 1 \
  --offline
```

Install the current checkout:

```bash
cargo install --path crates/why712-cli --locked
```

## Docker

```bash
docker pull ghcr.io/ptk030/why712:latest
docker run --rm --network none --read-only \
  ghcr.io/ptk030/why712:latest examples
```

For reproducible deployments, use a version or digest rather than `latest`.
