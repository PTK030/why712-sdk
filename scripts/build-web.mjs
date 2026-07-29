import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const webRoot = join(repositoryRoot, "web");
const appPublicRoot = join(webRoot, "public", "app");
const packageOutput = join(appPublicRoot, "pkg");
const exampleSource = join(repositoryRoot, "crates", "why712-core", "fixtures", "examples");
const exampleOutput = join(appPublicRoot, "examples");

for (const target of [packageOutput, exampleOutput]) {
  if (existsSync(target)) {
    rmSync(target, { recursive: true });
  }
}

const wasmPack = spawnSync(
  process.platform === "win32" ? "wasm-pack.exe" : "wasm-pack",
  [
    "build",
    "crates/why712-wasm",
    "--target",
    "web",
    "--out-dir",
    "../../web/public/app/pkg",
    "--out-name",
    "why712_wasm",
    "--release",
  ],
  {
    cwd: repositoryRoot,
    stdio: "inherit",
  },
);
if (wasmPack.status !== 0) {
  process.exit(wasmPack.status ?? 1);
}

mkdirSync(exampleOutput, { recursive: true });
const manifest = JSON.parse(readFileSync(join(exampleSource, "manifest.json"), "utf8"));
cpSync(join(exampleSource, "manifest.json"), join(exampleOutput, "manifest.json"));
for (const example of manifest) {
  const source = join(exampleSource, example.typedData);
  const destination = join(exampleOutput, example.typedData);
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(source, destination);
}

writeFileSync(
  join(appPublicRoot, "build.json"),
  `${JSON.stringify({ schemaVersion: 1, examples: manifest.length }, null, 2)}\n`,
);

const astroCli = join(repositoryRoot, "node_modules", "astro", "bin", "astro.mjs");
const astro = spawnSync(process.execPath, [astroCli, "build"], {
  cwd: repositoryRoot,
  stdio: "inherit",
});
if (astro.status !== 0) {
  process.exit(astro.status ?? 1);
}

console.log(`Built Astro site with ${manifest.length} reproducible examples.`);
