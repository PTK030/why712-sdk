import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("every application selector resolves to one unique HTML id", async () => {
  const [html, application] = await Promise.all([
    readFile(new URL("./index.html", import.meta.url), "utf8"),
    readFile(new URL("./app.js", import.meta.url), "utf8"),
  ]);
  const ids = [...html.matchAll(/id="([^"]+)"/gu)].map((match) => match[1]);
  const selectors = [...application.matchAll(/querySelector\("#([^"]+)"\)/gu)].map(
    (match) => match[1],
  );

  assert.equal(new Set(ids).size, ids.length, "HTML ids must be unique");
  assert.deepEqual(
    selectors.filter((selector) => !ids.includes(selector)),
    [],
    "application selectors must resolve",
  );
});
