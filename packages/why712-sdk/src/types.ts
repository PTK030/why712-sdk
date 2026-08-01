export type Hex = `0x${string}`;

export interface DiagnoseRequest {
  typedData?: object | string;
  transactionHash?: Hex;
  signature?: Hex;
  signer?: Hex;
  contract?: Hex;
  rpcUrl?: string;
  erc6492Validator?: Hex;
  chainId?: number;
}

export type Verdict = "MATCH" | "MISMATCH" | "INCONCLUSIVE";
export type Severity = "INFO" | "WARNING" | "ERROR";
export type Confidence = "CERTAIN" | "HIGH" | "HEURISTIC";

export interface DiagnosticFinding {
  code: string;
  severity: Severity;
  confidence: Confidence;
  message: string;
  expected?: string;
  actual?: string;
  remediation?: string;
  evidence?: string;
}

export interface ComputedValues {
  encodeType: string;
  typeHash: Hex;
  encodeData: Hex;
  hashStruct: Hex;
  domainSeparator: Hex;
  digest: Hex;
}

export interface DiagnosticContext {
  inputSource: "TYPED_DATA" | "WALLET_REQUEST" | "TRANSACTION";
  detectedStandard?:
    | "EIP2612"
    | "PERMIT2_ALLOWANCE"
    | "PERMIT2_SIGNATURE_TRANSFER"
    | "ERC1271"
    | "ERC6492";
  chainId?: string;
  blockNumber?: number;
  blockTimestamp?: number;
  protocol?: Record<string, unknown>;
  erc6492Wrapper?: Record<string, unknown>;
  transaction?: Record<string, unknown>;
}

export interface DiagnosticReport {
  schemaVersion: 2;
  verdict: Verdict;
  computed: ComputedValues;
  expectedSigner?: Hex;
  recoveredSigner?: Hex;
  findings: DiagnosticFinding[];
  context?: DiagnosticContext;
}

export interface ExampleSummary {
  id: string;
  title: string;
  fault?: string;
  typedData: string;
  signature?: Hex;
  signer?: Hex;
  expectedVerdict: Verdict;
  expectedRootCode?: string;
}
