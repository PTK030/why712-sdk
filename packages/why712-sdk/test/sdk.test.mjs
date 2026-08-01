import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  diagnose,
  diagnoseOffline,
  examples,
  reportToMarkdown,
} from "../dist/node.js";

const fixtureUrl = new URL(
  "../../../crates/why712-core/fixtures/examples/valid-mail.json",
  import.meta.url,
);
const signature = "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c";
const signer = "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826";

test("Node SDK executes the Rust engine and returns a typed report shape", async () => {
  const typedData = JSON.parse(await readFile(fixtureUrl, "utf8"));
  const report = await diagnoseOffline({ typedData, signature, signer });

  assert.equal(report.schemaVersion, 2);
  assert.equal(report.verdict, "MATCH");
  assert.deepEqual(report.findings, []);
  assert.match(report.computed.digest, /^0x[0-9a-f]{64}$/i);
  assert.match(await reportToMarkdown(report), /MATCH/);
});

test("SDK exposes the core's reproducible example catalog", async () => {
  const manifest = await examples();
  assert.ok(manifest.some((example) => example.id === "valid-mail"));
});

test("Node RPC mode uses global fetch and preserves offline diagnosis on failure", async () => {
  const typedData = JSON.parse(await readFile(fixtureUrl, "utf8"));
  const originalFetch = globalThis.fetch;
  let calls = 0;
  globalThis.fetch = async (request) => {
    calls += 1;
    assert.ok(request instanceof Request);
    return new Response(
      JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        error: { code: -32000, message: "fixture RPC refusal" },
      }),
      { status: 200, headers: { "content-type": "application/json" } },
    );
  };

  try {
    const report = await diagnose({
      typedData,
      signature,
      signer,
      contract: "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC",
      rpcUrl: "https://rpc.invalid.test",
    });
    assert.equal(report.verdict, "MATCH");
    assert.ok(calls > 0);
  } finally {
    globalThis.fetch = originalFetch;
  }
});
