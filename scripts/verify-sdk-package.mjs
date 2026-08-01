import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const archive = process.argv[2];
if (!archive) {
  throw new Error("usage: node scripts/verify-sdk-package.mjs <package.tgz>");
}

const temporaryRoot = mkdtempSync(join(tmpdir(), "why712-sdk-package-"));
try {
  const extracted = spawnSync("tar", ["-xf", resolve(archive), "-C", temporaryRoot], {
    stdio: "inherit",
  });
  if (extracted.status !== 0) {
    process.exit(extracted.status ?? 1);
  }

  const extractedPackage = join(temporaryRoot, "package");
  const manifest = JSON.parse(readFileSync(join(extractedPackage, "package.json"), "utf8"));
  if (manifest.license !== "MIT") {
    throw new Error(`packed SDK license is ${manifest.license ?? "missing"}, expected MIT`);
  }
  for (const relative of ["LICENSE", "dist/browser/LICENSE", "dist/node/LICENSE"]) {
    const license = readFileSync(join(extractedPackage, relative), "utf8");
    if (!license.startsWith("MIT License\n")) {
      throw new Error(`${relative} does not contain the MIT license`);
    }
  }

  const modulesRoot = join(temporaryRoot, "node_modules");
  mkdirSync(modulesRoot);
  renameSync(extractedPackage, join(modulesRoot, "why712"));
  const imported = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "--eval",
      "import { examples } from 'why712'; const xs = await examples(); if (!xs.some((x) => x.id === 'valid-mail')) process.exit(2);",
    ],
    { cwd: temporaryRoot, stdio: "inherit" },
  );
  if (imported.status !== 0) {
    throw new Error(`packed SDK import failed with exit code ${imported.status ?? "unknown"}`);
  }
  console.log("Verified MIT metadata, package-name resolution, and the Rust/WebAssembly engine.");
} finally {
  rmSync(temporaryRoot, { recursive: true, force: true });
}
