---
name: research-eip712-fixture
description: Research, generate, and verify reproducible EIP-712 fixtures for why712 using primary specifications and an implementation independent of the Rust core. Use for official vectors, real-world protocol cases, regressions, or new hypothesis examples.
---

# Research an EIP-712 fixture

1. Read the vault protocol plus `docs/testing.md` and the existing fixture manifest.
2. Define the exact claim the fixture proves and the expected finding. Prefer the applicable
   EIP/ERC, protocol contract repository, and official library documentation.
3. Inspect the project's exact pinned Alloy sources so the test matches the API actually built.
   If sources disagree or current facts cannot be established, stop guessing and record an open
   question.
4. Generate the expected separator, struct hash, digest, and signature with an independent
   pinned implementation such as viem or ethers in an isolated temporary directory. Never use
   why712 to generate its own oracle.
5. Record source URLs, upstream versions or commit hashes, generator version, and enough input
   data to reproduce the result. Do not commit temporary dependency trees or secrets.
6. Add the smallest fixture that captures the behavior, its manifest entry, integration test,
   and snapshot. Include a nearby counterexample if it guards against a plausible false hit.
7. Run the required native and wasm checks from `docs/testing.md`; inspect snapshot changes.
8. Promote stable research conclusions to public docs, then update vault status and journal.

Never use live production credentials, private keys, unverifiable explorer output, or a fixture
whose expected result was copied from the implementation under test.
