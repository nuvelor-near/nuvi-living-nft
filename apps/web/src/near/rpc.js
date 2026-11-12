import { appConfig } from "../config";

function encodeArgs(args) {
  const json = JSON.stringify(args ?? {});
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function decodeResult(bytes) {
  const json = new TextDecoder().decode(new Uint8Array(bytes || []));
  return json ? JSON.parse(json) : null;
}

let requestId = 0;

export async function rpc(method, params) {
  requestId += 1;
  const response = await fetch(appConfig.rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: `nuvi-${requestId}`,
      method,
      params,
    }),
  });

  if (!response.ok) {
    throw new Error(`NEAR RPC HTTP ${response.status}`);
  }

  const payload = await response.json();
  if (payload.error) {
    const cause =
      payload.error?.data?.cause?.name ||
      payload.error?.cause?.name ||
      payload.error?.message ||
      "Unknown RPC error";
    throw new Error(String(cause));
  }

  return payload.result;
}

export async function viewFunction(contractId, methodName, args = {}) {
  if (!contractId) throw new Error("Contract ID is not configured");

  const result = await rpc("query", {
    request_type: "call_function",
    finality: "final",
    account_id: contractId,
    method_name: methodName,
    args_base64: encodeArgs(args),
  });

  return decodeResult(result?.result);
}
