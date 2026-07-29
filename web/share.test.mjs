import assert from "node:assert/strict";
import test from "node:test";

import { decodeShareState, encodeShareState } from "./share.js";

test("share fragments round-trip unicode input without leaking unknown fields", () => {
  const input = {
    typedData: '{"message":{"note":"Zażółć 🧪"}}',
    signature: "0x1234",
    chainId: "1",
    ignored: "do not include",
  };

  const encoded = encodeShareState(input);
  assert.doesNotMatch(encoded, /[+/=]/u);
  assert.deepEqual(decodeShareState(`#${encoded}`), {
    typedData: input.typedData,
    signature: input.signature,
    chainId: input.chainId,
  });
});

test("empty fragments do not produce state", () => {
  assert.equal(decodeShareState(""), null);
  assert.equal(decodeShareState("#"), null);
});
