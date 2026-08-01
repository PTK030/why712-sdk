// wasm-pack emits a CommonJS loader that synchronously instantiates the embedded WASM module.
// @ts-expect-error The generated module exists in dist only during a complete SDK build.
import nodeBindings from "./node/why712_wasm.js";

import type { DiagnoseRequest, DiagnosticReport, ExampleSummary } from "./types.js";

export type {
  ComputedValues,
  Confidence,
  DiagnoseRequest,
  DiagnosticContext,
  DiagnosticFinding,
  DiagnosticReport,
  ExampleSummary,
  Hex,
  Severity,
  Verdict,
} from "./types.js";

interface NodeBindings {
  diagnose(input: DiagnoseRequest): Promise<DiagnosticReport>;
  diagnose_offline(input: DiagnoseRequest): DiagnosticReport;
  examples(): ExampleSummary[];
  report_to_markdown(report: DiagnosticReport): string;
}

const rust = nodeBindings as NodeBindings;

/** The Node loader instantiates WebAssembly during module import. */
export async function initialize(): Promise<void> {}

/** Runs deterministic diagnosis without contacting an RPC endpoint. */
export async function diagnoseOffline(input: DiagnoseRequest): Promise<DiagnosticReport> {
  return rust.diagnose_offline(input);
}

/** Acquires optional RPC evidence through global fetch and runs the Rust diagnosis engine. */
export async function diagnose(input: DiagnoseRequest): Promise<DiagnosticReport> {
  return rust.diagnose(input);
}

/** Renders a report as GitHub-flavored Markdown. */
export async function reportToMarkdown(report: DiagnosticReport): Promise<string> {
  return rust.report_to_markdown(report);
}

/** Returns the reproducible examples bundled into the Rust engine. */
export async function examples(): Promise<ExampleSummary[]> {
  return rust.examples();
}
