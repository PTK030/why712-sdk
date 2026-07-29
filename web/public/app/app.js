import initWasm, {
  diagnose,
  diagnose_offline as diagnoseOffline,
  report_to_markdown as reportToMarkdown,
} from "./pkg/why712_wasm.js";
import { decodeShareState, encodeShareState } from "./share.js";

const elements = {
  form: document.querySelector("#diagnose-form"),
  typedData: document.querySelector("#typed-data"),
  signature: document.querySelector("#signature"),
  signer: document.querySelector("#signer"),
  contract: document.querySelector("#contract"),
  rpcUrl: document.querySelector("#rpc-url"),
  chainId: document.querySelector("#chain-id"),
  diagnoseButton: document.querySelector("#diagnose-button"),
  clearButton: document.querySelector("#clear-button"),
  formStatus: document.querySelector("#form-status"),
  engineStatus: document.querySelector("#engine-status"),
  exampleList: document.querySelector("#example-list"),
  emptyResult: document.querySelector("#empty-result"),
  report: document.querySelector("#report"),
  verdictCard: document.querySelector("#verdict-card"),
  verdictLabel: document.querySelector("#verdict-label"),
  verdictSummary: document.querySelector("#verdict-summary"),
  findingCount: document.querySelector("#finding-count"),
  findingList: document.querySelector("#finding-list"),
  signerGrid: document.querySelector("#signer-grid"),
  computedGrid: document.querySelector("#computed-grid"),
  copyMarkdown: document.querySelector("#copy-markdown"),
  copyLink: document.querySelector("#copy-link"),
  toast: document.querySelector("#toast"),
};

let engineReady = false;
let currentReport = null;
let toastTimer;

elements.form.addEventListener("submit", handleSubmit);
elements.clearButton.addEventListener("click", clearForm);
elements.copyMarkdown.addEventListener("click", copyMarkdownReport);
elements.copyLink.addEventListener("click", copyShareLink);

await start();

async function start() {
  restoreShareState();
  await Promise.allSettled([loadEngine(), loadExamples()]);
}

async function loadEngine() {
  try {
    await initWasm();
    engineReady = true;
    elements.engineStatus.textContent = "Engine ready";
    elements.engineStatus.classList.add("ready");
  } catch (error) {
    elements.engineStatus.textContent = "Engine unavailable";
    setFormStatus(`Could not load the diagnostic engine: ${errorMessage(error)}`);
  }
}

async function loadExamples() {
  try {
    const response = await fetch("./examples/manifest.json");
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const manifest = await response.json();
    elements.exampleList.replaceChildren(
      ...manifest.map((example) => {
        const button = document.createElement("button");
        button.className = "example-button";
        button.type = "button";
        button.textContent = example.title;
        button.addEventListener("click", () => loadExample(example));
        return button;
      }),
    );
  } catch (error) {
    const message = document.createElement("span");
    message.className = "loading-label";
    message.textContent = `Examples unavailable: ${errorMessage(error)}`;
    elements.exampleList.replaceChildren(message);
  }
}

async function loadExample(example) {
  try {
    const response = await fetch(`./examples/${encodeURIComponent(example.typedData)}`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    elements.typedData.value = JSON.stringify(await response.json(), null, 2);
    elements.signature.value = example.signature ?? "";
    elements.signer.value = example.signer ?? "";
    elements.contract.value = "";
    elements.rpcUrl.value = "";
    elements.chainId.value = example.hints?.chainIds?.[0] ?? "";
    setFormStatus("");
    showToast(`Loaded: ${example.title}`);
    elements.typedData.focus();
  } catch (error) {
    setFormStatus(`Could not load the example: ${errorMessage(error)}`);
  }
}

async function handleSubmit(event) {
  event.preventDefault();
  setFormStatus("");
  if (!engineReady) {
    setFormStatus("The WebAssembly engine is still loading.");
    return;
  }

  let request;
  try {
    request = buildRequest();
  } catch (error) {
    setFormStatus(errorMessage(error));
    return;
  }

  setBusy(true);
  try {
    const report =
      request.rpcUrl && request.contract
        ? await diagnose(request)
        : diagnoseOffline(request);
    currentReport = report;
    renderReport(report);
  } catch (error) {
    setFormStatus(errorMessage(error));
  } finally {
    setBusy(false);
  }
}

function buildRequest() {
  const typedData = elements.typedData.value.trim();
  if (!typedData) {
    throw new Error("Typed data is required.");
  }
  try {
    JSON.parse(typedData);
  } catch (error) {
    throw new Error(`Typed data is not valid JSON: ${error.message}`, { cause: error });
  }

  const request = compact({
    typedData,
    signature: elements.signature.value.trim(),
    signer: elements.signer.value.trim(),
    contract: elements.contract.value.trim(),
    rpcUrl: elements.rpcUrl.value.trim(),
  });

  const chainId = elements.chainId.value.trim();
  if (chainId) {
    const parsed = Number(chainId);
    if (!Number.isSafeInteger(parsed) || parsed < 0) {
      throw new Error("Candidate chain ID must be a non-negative safe integer.");
    }
    request.chainId = parsed;
  }
  if (request.rpcUrl && !request.contract) {
    throw new Error("An RPC endpoint requires a verifying contract.");
  }
  return request;
}

function renderReport(report) {
  elements.emptyResult.hidden = true;
  elements.report.hidden = false;

  const verdict = report.verdict.toLowerCase();
  elements.verdictCard.className = `verdict-card ${verdict}`;
  elements.verdictLabel.textContent = report.verdict;
  elements.verdictSummary.textContent = verdictSummary(report);

  renderFindings(report.findings);
  renderValueGrid(elements.signerGrid, [
    ["Expected", report.expectedSigner ?? "Not supplied"],
    ["Recovered", report.recoveredSigner ?? "Not available"],
  ]);
  renderValueGrid(elements.computedGrid, [
    ["encodeType", report.computed.encodeType],
    ["typeHash", report.computed.typeHash],
    ["encodeData", report.computed.encodeData],
    ["hashStruct", report.computed.structHash],
    ["domainSeparator", report.computed.domainSeparator],
    ["digest", report.computed.digest],
  ]);
  elements.report.scrollIntoView({ behavior: "smooth", block: "start" });
}

function renderFindings(findings) {
  elements.findingCount.textContent = `${findings.length} finding${findings.length === 1 ? "" : "s"}`;
  if (findings.length === 0) {
    const clean = document.createElement("div");
    clean.className = "clean-finding";
    const message = document.createElement("p");
    message.textContent = "Every supplied verification input agrees.";
    clean.append(message);
    elements.findingList.replaceChildren(clean);
    return;
  }

  elements.findingList.replaceChildren(...findings.map(renderFinding));
}

function renderFinding(finding) {
  const card = document.createElement("article");
  card.className = `finding ${finding.severity.toLowerCase()}`;

  const header = document.createElement("div");
  header.className = "finding-header";
  const code = document.createElement("span");
  code.className = "finding-code";
  code.textContent = finding.code;
  const confidence = document.createElement("span");
  confidence.className = "finding-confidence";
  confidence.textContent = `${finding.severity} · ${finding.confidence}`;
  header.append(code, confidence);

  const message = document.createElement("p");
  message.textContent = finding.message;
  card.append(header, message);

  if (finding.expected !== undefined || finding.actual !== undefined) {
    const diff = document.createElement("div");
    diff.className = "diff-grid";
    diff.append(
      renderDiffValue("Expected", finding.expected ?? "Not available"),
      renderDiffValue("Actual", finding.actual ?? "Not available"),
    );
    card.append(diff);
  }

  if (finding.remediation) {
    const remediation = document.createElement("p");
    remediation.className = "remediation";
    remediation.textContent = `Fix: ${finding.remediation}`;
    card.append(remediation);
  }
  return card;
}

function renderDiffValue(label, value) {
  const wrapper = document.createElement("div");
  wrapper.className = "diff-value";
  const heading = document.createElement("span");
  heading.textContent = label;
  const code = document.createElement("code");
  code.textContent = value;
  wrapper.append(heading, code);
  return wrapper;
}

function renderValueGrid(container, values) {
  container.replaceChildren(
    ...values.map(([label, value]) => {
      const row = document.createElement("div");
      const term = document.createElement("dt");
      term.textContent = label;
      const description = document.createElement("dd");
      const code = document.createElement("code");
      code.textContent = value;
      description.append(code);
      row.append(term, description);
      return row;
    }),
  );
}

async function copyMarkdownReport() {
  if (!currentReport) {
    return;
  }
  try {
    await copyText(reportToMarkdown(currentReport));
    showToast("Markdown report copied");
  } catch (error) {
    setFormStatus(`Could not copy the report: ${errorMessage(error)}`);
  }
}

async function copyShareLink() {
  try {
    const state = readFormState();
    if (!state.typedData) {
      throw new Error("Add typed data before copying a link.");
    }
    const url = new URL(window.location.href);
    url.hash = encodeShareState(state);
    history.replaceState(null, "", url);
    await copyText(url.href);
    showToast("Fragment-only share link copied");
  } catch (error) {
    setFormStatus(`Could not copy the link: ${errorMessage(error)}`);
  }
}

function restoreShareState() {
  try {
    const state = decodeShareState(window.location.hash);
    if (state) {
      writeFormState(state);
      showToast("Inputs restored from the URL fragment");
    }
  } catch {
    setFormStatus("The URL fragment does not contain a valid why712 request.");
  }
}

function readFormState() {
  return compact({
    typedData: elements.typedData.value,
    signature: elements.signature.value,
    signer: elements.signer.value,
    contract: elements.contract.value,
    rpcUrl: elements.rpcUrl.value,
    chainId: elements.chainId.value,
  });
}

function writeFormState(state) {
  elements.typedData.value = state.typedData ?? "";
  elements.signature.value = state.signature ?? "";
  elements.signer.value = state.signer ?? "";
  elements.contract.value = state.contract ?? "";
  elements.rpcUrl.value = state.rpcUrl ?? "";
  elements.chainId.value = state.chainId ?? "";
}

function clearForm() {
  elements.form.reset();
  currentReport = null;
  elements.report.hidden = true;
  elements.emptyResult.hidden = false;
  elements.formStatus.textContent = "";
  history.replaceState(null, "", `${window.location.pathname}${window.location.search}`);
  elements.typedData.focus();
}

function verdictSummary(report) {
  switch (report.verdict) {
    case "MATCH":
      return "The supplied signature recovers the expected signer for this canonical digest.";
    case "MISMATCH":
      return "The supplied inputs do not verify. The proven or directly observed differences are listed below.";
    default:
      return "The available inputs are insufficient for a definitive match. Add a signer, signature, or chain evidence.";
  }
}

function compact(value) {
  return Object.fromEntries(
    Object.entries(value).filter(([, fieldValue]) => fieldValue !== ""),
  );
}

async function copyText(text) {
  await navigator.clipboard.writeText(text);
}

function setBusy(busy) {
  elements.diagnoseButton.disabled = busy;
  elements.diagnoseButton.firstElementChild.textContent = busy
    ? "Diagnosing…"
    : "Diagnose signature";
}

function setFormStatus(message) {
  elements.formStatus.textContent = message;
}

function showToast(message) {
  clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.add("visible");
  toastTimer = setTimeout(() => elements.toast.classList.remove("visible"), 2200);
}

function errorMessage(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
