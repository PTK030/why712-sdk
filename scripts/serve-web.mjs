import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, resolve, sep } from "node:path";

const webRoot = resolve("web");
const port = Number(process.env.WHY712_WEB_PORT ?? 4173);
const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".wasm", "application/wasm"],
]);

const server = createServer((request, response) => {
  try {
    const requestUrl = new URL(request.url ?? "/", "http://localhost");
    const relativePath = decodeURIComponent(requestUrl.pathname).replace(/^\/+/u, "");
    const candidate = resolve(webRoot, relativePath || "index.html");
    if (candidate !== webRoot && !candidate.startsWith(`${webRoot}${sep}`)) {
      throw new Error("path escapes web root");
    }
    const filePath = statSync(candidate).isDirectory() ? resolve(candidate, "index.html") : candidate;
    response.writeHead(200, {
      "Cache-Control": "no-store",
      "Content-Type": contentTypes.get(extname(filePath)) ?? "application/octet-stream",
    });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    response.end("Not found");
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`why712 web demo: http://127.0.0.1:${port}/`);
});
