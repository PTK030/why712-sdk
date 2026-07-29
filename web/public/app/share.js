// RPC URLs commonly contain project credentials, so fragment links never include them.
const SHARE_FIELDS = ["typedData", "signature", "signer", "contract", "chainId"];

export function encodeShareState(state) {
  const bytes = new TextEncoder().encode(JSON.stringify(pickShareFields(state)));
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/u, "");
}

export function decodeShareState(fragment) {
  const encoded = fragment.replace(/^#/u, "");
  if (!encoded) {
    return null;
  }
  const padded = encoded.replaceAll("-", "+").replaceAll("_", "/").padEnd(
    Math.ceil(encoded.length / 4) * 4,
    "=",
  );
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return pickShareFields(JSON.parse(new TextDecoder().decode(bytes)));
}

function pickShareFields(state) {
  const result = {};
  for (const field of SHARE_FIELDS) {
    const value = state?.[field];
    if (typeof value === "string" && value.length > 0) {
      result[field] = value;
    }
  }
  return result;
}
