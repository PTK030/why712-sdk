import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const files = {
  tokens: new URL("../styles/tokens.css", import.meta.url),
  landing: new URL("../pages/index.astro", import.meta.url),
  playground: new URL("../pages/app.astro", import.meta.url),
  docs: new URL("../styles/docs.css", import.meta.url),
  site: new URL("../styles/site.css", import.meta.url),
  landingCss: new URL("../styles/landing.css", import.meta.url),
  playgroundCss: new URL("../styles/playground.css", import.meta.url),
};

const source = Object.fromEntries(
  await Promise.all(
    Object.entries(files).map(async ([name, url]) => [name, await readFile(url, "utf8")]),
  ),
);

test("all three web surfaces share the same design tokens", () => {
  assert.match(source.site, /@import "\.\/tokens\.css";/);
  assert.match(source.docs, /@import "\.\/tokens\.css";/);
  assert.match(source.landing, /SiteHeader/);
  assert.match(source.landing, /SiteFooter/);
  assert.match(source.playground, /SiteHeader/);
  assert.match(source.playground, /SiteFooter/);
});

test("the visual language excludes decorative effects that obscure hierarchy", () => {
  const visualSource = [
    source.docs,
    source.site,
    source.landingCss,
    source.playgroundCss,
  ].join("\n");

  for (const forbidden of [
    /(?:linear|radial|conic)-gradient\s*\(/i,
    /backdrop-filter\s*:/i,
    /filter\s*:\s*blur\s*\(/i,
    /animation\s*:/i,
  ]) {
    assert.doesNotMatch(visualSource, forbidden);
  }
});

test("the core text colors meet WCAG AA contrast", () => {
  const lightTokens = source.tokens.split('[data-theme="dark"]')[0];
  const tokens = Object.fromEntries(
    [...lightTokens.matchAll(/--([\w-]+):\s*(#[0-9a-f]{6});/gi)].map((match) => [
      match[1],
      match[2],
    ]),
  );

  assert.ok(contrast(tokens["color-ink"], tokens["color-canvas"]) >= 4.5);
  assert.ok(contrast(tokens["color-ink-soft"], tokens["color-canvas"]) >= 4.5);
  assert.ok(contrast(tokens["color-ink-faint"], tokens["color-canvas"]) >= 4.5);
  assert.ok(contrast(tokens["color-code-text"], tokens["color-code"]) >= 4.5);
  assert.ok(contrast(tokens["color-code-muted"], tokens["color-code"]) >= 4.5);
  assert.ok(contrast(tokens["color-accent-ink"], tokens["color-accent"]) >= 4.5);
});

test("the landing page leads with the user problem and real product evidence", () => {
  assert.match(source.landing, /Find out why the signature does not verify\./);
  assert.match(source.landing, /DOMAIN_CHAIN_ID_MISMATCH/);
  assert.match(source.landing, /Your input stays local by default\./);
  assert.doesNotMatch(source.landing, /trusted by|root cause found|AI-powered/i);
});

function contrast(foreground, background) {
  const lighter = Math.max(luminance(foreground), luminance(background));
  const darker = Math.min(luminance(foreground), luminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

function luminance(hex) {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) =>
      channel <= 0.04045
        ? channel / 12.92
        : ((channel + 0.055) / 1.055) ** 2.4,
    );

  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}
