import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

const repository = "why712";

export default defineConfig({
  site: "https://ptk030.github.io",
  base: `/${repository}`,
  outDir: "./web/dist",
  publicDir: "./web/public",
  srcDir: "./web/src",
  integrations: [
    starlight({
      title: "why712",
      description: "Diagnose EIP-712 signature verification failures.",
      logo: {
        src: "./web/src/assets/logo.svg",
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/PTK030/why712",
        },
      ],
      customCss: ["./web/src/styles/docs.css"],
      sidebar: [
        {
          label: "Start here",
          items: [
            { label: "Overview", slug: "docs/guides/overview" },
            { label: "Getting started", slug: "docs/guides/getting-started" },
            { label: "Browser playground", slug: "docs/guides/browser" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Use contract evidence", slug: "docs/guides/rpc" },
            { label: "Use why712 in CI", slug: "docs/guides/ci" },
            { label: "Privacy and security", slug: "docs/guides/privacy" },
          ],
        },
        {
          label: "Understand",
          items: [
            { label: "The EIP-712 pipeline", slug: "docs/concepts/eip712-pipeline" },
            { label: "How to read a report", slug: "docs/concepts/report" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI", slug: "docs/guides/cli" },
            { label: "Finding codes", slug: "docs/reference/findings" },
            { label: "Report schema", slug: "docs/reference/report-schema" },
            { label: "Limits", slug: "docs/reference/limits" },
          ],
        },
        {
          label: "Project",
          items: [
            { label: "Commercial licensing", slug: "docs/commercial-license" },
          ],
        },
      ],
    }),
  ],
});
