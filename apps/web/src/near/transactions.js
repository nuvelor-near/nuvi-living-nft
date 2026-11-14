import { emitWalletDiagnostic } from "../wallet/diagnostics.js";
import { bindMeteorMobileNearConnectOpenBridge } from "../wallet/meteorExecutorPatch.js";

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

  const transaction = {
    receiverId,
    actions: [
      functionCallAction({
        methodName,
        args,
        gasTgas,
        deposit,
      }),
    ],
  };

  let wallet = await connector.wallet();
  bindMeteorMobileNearConnectOpenBridge(wallet);
  const walletId = wallet?.manifest?.id || "unknown";
  emitWalletDiagnostic("tx-sign-start", {
    walletId,
    method: methodName,
    receiver: receiverId,
    gasTgas,
    deposit: String(deposit),
  });

  if (typeof wallet?.signAndSendTransaction !== "function") {
    emitWalletDiagnostic("tx-sign-unavailable", { walletId, method: methodName });
    throw new Error("Connected wallet cannot sign NEAR transactions");
  }

  let result;
  try {
    result = await wallet.signAndSendTransaction(transaction);
  } catch (cause) {
    emitWalletDiagnostic("tx-sign-error", {
      walletId,
      method: methodName,
      message: cause instanceof Error ? cause.message : String(cause),
    });
    throw cause;
  }

  if (result != null) {
    emitWalletDiagnostic("tx-sign-result", {
      walletId,
      method: methodName,
      result: "non-null",
    });
    return result;
  }

  emitWalletDiagnostic("tx-sign-null-result", {
    walletId,
    method: methodName,
  });

  // A sandbox wallet can restore the public account identity while its
  // internal signing session has expired. Reconnect the same wallet once and
  // retry the exact same Testnet transaction. Non-null results are never
  // retried, preventing this recovery path from duplicating a transaction.
  if (!walletId || walletId === "unknown" || typeof connector.connect !== "function") {
    throw new Error(
      "Wallet signing session expired. Disconnect and connect the wallet again.",
    );
  }

  emitWalletDiagnostic("tx-sign-reconnect", { walletId, method: methodName });
  const reconnected = await connector.connect({ walletId });
  wallet = reconnected || (await connector.wallet(walletId));
  bindMeteorMobileNearConnectOpenBridge(wallet);
  if (typeof wallet?.signAndSendTransaction !== "function") {
    throw new Error("Reconnected wallet cannot sign NEAR transactions");
  }

  try {
    result = await wallet.signAndSendTransaction(transaction);
  } catch (cause) {
    emitWalletDiagnostic("tx-sign-retry-error", {
      walletId,
      method: methodName,
      message: cause instanceof Error ? cause.message : String(cause),
    });
    throw cause;
  }

  if (result == null) {
    emitWalletDiagnostic("tx-sign-retry-null", { walletId, method: methodName });
    throw new Error(
      "Wallet signing session could not be restored. Disconnect and connect the wallet again.",
    );
  }

  emitWalletDiagnostic("tx-sign-retry-result", {
    walletId,
    method: methodName,
    result: "non-null",
  });
  return result;
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
