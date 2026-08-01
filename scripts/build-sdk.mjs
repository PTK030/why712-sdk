import { spawnSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const sdkRoot = join(repositoryRoot, "packages", "why712-sdk");
const outputRoot = join(sdkRoot, "dist");

if (existsSync(outputRoot)) {
  rmSync(outputRoot, { recursive: true });
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const wasmPack = process.platform === "win32" ? "wasm-pack.exe" : "wasm-pack";
for (const target of ["web", "nodejs"]) {
  const output = target === "web" ? "browser" : "node";
  const generatedRoot = join(outputRoot, output);
  run(wasmPack, [
    "build",
    "crates/why712-wasm",
    "--target",
    target,
    "--out-dir",
    `../../packages/why712-sdk/dist/${output}`,
    "--out-name",
    "why712_wasm",
    "--release",
  ]);
  // wasm-pack protects its standalone output with `*`; the SDK intentionally embeds that
  // output, so retaining the generated ignore file would silently omit the Rust engine from
  // the npm tarball.
  rmSync(join(generatedRoot, ".gitignore"), { force: true });
}

const tsc = join(repositoryRoot, "node_modules", "typescript", "bin", "tsc");
run(process.execPath, [tsc, "--project", join(sdkRoot, "tsconfig.json")]);
