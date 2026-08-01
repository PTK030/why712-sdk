---
name: keep-docs-current
description: Keep why712 documentation synchronized with implementation changes. Use when planning, implementing, reviewing, or preparing a PR or release for a feature, public behavior, API, CLI, WASM, web, FindingCode, workflow, dependency, toolchain, security boundary, or deprecation change.
---

# Keep why712 docs current

Treat documentation as part of the change, not as follow-up work. Audit every affected
documentation surface before declaring a feature, PR, or release complete.

## Audit the change

1. Inspect the plan, issue, branch diff, and PR description to identify changed behavior.
2. List the audiences affected: library users, CLI users, browser users, contributors,
   operators, release managers, and agents.
3. Map each durable fact to its owning documentation surface:

| Change | Required surface |
|---|---|
| Architecture, process, testing, development, or release workflow | Matching file under `docs/` |
| New or changed `FindingCode` | `docs/diagnostics.md` in the same change |
| User-facing CLI, WASM, or TypeScript SDK behavior | `README.md`, package README, and matching canonical `docs/` file |
| Product positioning, install path, or quick start | `README.md` |
| User-facing canonical README content | Relevant translated READMEs |
| Security, privacy, or trust boundary | `SECURITY.md` and any affected public guide |
| Contributor or agent workflow | `CONTRIBUTING.md`, `AGENTS.md`, or a focused project skill |
| Notable user-visible release change | `CHANGELOG.md` |

Root `docs/` remains the canonical source for system and process knowledge. The repository and
package READMEs are task-oriented user guidance; link to canonical docs instead of copying large
sections blindly.

## Update documentation with the change

- Update affected docs in the same branch and PR as the implementation.
- Describe the current behavior directly. Do not leave promises, TODOs, or migration notes that
  are not actionable.
- Keep commands, paths, option names, examples, outputs, and version claims consistent with the
  implementation.
- When no documentation needs to change, write `Documentation impact: none` in the PR and give
  a concrete reason. Do not touch a doc solely to satisfy a checkbox.
- Promote durable conclusions to `docs/`; keep temporary investigation state in `.vault/`.

## Verify the result

1. Review the entire diff for stale claims and newly undocumented behavior.
2. Run the checks appropriate to the changed surface:
   - TypeScript SDK: `pnpm build:sdk`, `pnpm check:sdk`, `pnpm test:sdk`, and packed-archive verification;
   - Rust behavior or examples: the relevant narrow test, then the repository quality gates;
   - `FindingCode`: the diagnostic catalog contract and affected snapshots.
3. Confirm links, commands, paths, code snippets, and example output against the current tree.
4. In the PR summary, list the documentation updated or the explicit no-impact rationale.
5. Record material progress in `.vault/STATUS.md` and `.vault/JOURNAL.md`.
