export const TGAS = 1_000_000_000_000n;

export function functionCallAction({
  methodName,
  args = {},
  gasTgas = 100,
  deposit = "0",
}) {
  return {
    type: "FunctionCall",
    params: {
      methodName,
      args,
      gas: (BigInt(gasTgas) * TGAS).toString(),
      deposit: String(deposit),
    },
  };
}

export async function signFunctionCall(
  connector,
  {
    receiverId,
    methodName,
    args = {},
    gasTgas = 100,
    deposit = "0",
  },
) {
  if (!connector) throw new Error("Wallet is not connected");
  if (!receiverId) throw new Error("Receiver contract is not configured");

  const wallet = await connector.wallet();
  if (typeof wallet?.signAndSendTransaction !== "function") {
    throw new Error("Connected wallet cannot sign NEAR transactions");
  }

  return wallet.signAndSendTransaction({
    receiverId,
    actions: [
      functionCallAction({
        methodName,
        args,
        gasTgas,
        deposit,
      }),
    ],
  });
}

export function extractTransactionHashes(value) {
  const hashes = new Set();

  function visit(item) {
    if (!item) return;
    if (Array.isArray(item)) {
      item.forEach(visit);
      return;
    }
    if (typeof item !== "object") return;

    const candidates = [
      item?.transaction?.hash,
      item?.transaction_outcome?.id,
      item?.final_execution_outcome?.transaction?.hash,
      item?.result?.transaction?.hash,
    ];
    candidates.filter(Boolean).forEach((hash) => hashes.add(String(hash)));

    if (item.results) visit(item.results);
  }

  visit(value);
  return [...hashes];
}
