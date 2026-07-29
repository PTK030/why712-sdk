import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";

const outputRoot = resolve("web", "dist");
const deploymentBase = "/why712";
const htmlFiles = walk(outputRoot).filter((path) => extname(path) === ".html");
const failures = [];

for (const htmlFile of htmlFiles) {
  const html = readFileSync(htmlFile, "utf8");
  for (const match of html.matchAll(/\b(?:href|src)="([^"]+)"/gu)) {
    const reference = match[1];
    if (
      reference.startsWith("#") ||
      reference.startsWith("data:") ||
      reference.startsWith("mailto:")
    ) {
      continue;
    }

    const url = new URL(reference, `https://example.test${deploymentBase}/${relative(outputRoot, htmlFile).replaceAll("\\", "/")}`);
    if (url.origin !== "https://example.test") {
      continue;
    }
    if (url.pathname !== deploymentBase && !url.pathname.startsWith(`${deploymentBase}/`)) {
      failures.push(`${relative(outputRoot, htmlFile)} -> ${reference} escapes ${deploymentBase}`);
      continue;
    }

    const deployedPath = decodeURIComponent(url.pathname.slice(deploymentBase.length)).replace(
      /^\/+/u,
      "",
    );
    let candidate = join(outputRoot, deployedPath);
    if (!extname(candidate) || url.pathname.endsWith("/")) {
      candidate = join(candidate, "index.html");
    }
    if (!existsSync(candidate) || !statSync(candidate).isFile()) {
      failures.push(`${relative(outputRoot, htmlFile)} -> ${reference}`);
    }
  }
}

if (failures.length > 0) {
  console.error(`Broken internal references:\n${failures.map((item) => `- ${item}`).join("\n")}`);
  process.exit(1);
}

console.log(`Checked internal references in ${htmlFiles.length} generated HTML files.`);

function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}
