import { appConfig, contractsConfigured } from "../config";
import { viewAccessKeyList, viewAccount, viewFunction } from "./rpc";
import { signFunctionCall } from "./transactions";

export const NUVE_DECIMALS = 4;
export const NUVE_UNIT = 10_000n;

const ACTIONS = Object.freeze({
  feed: { amount: 5n * NUVE_UNIT, label: "Feed" },
  work_a: { amount: 10n * NUVE_UNIT, label: "Work A" },
  work_b: { amount: 10n * NUVE_UNIT, label: "Work B" },
});

export function actionAmountAtomic(action) {
  const entry = ACTIONS[action];
  if (!entry) throw new Error(`Unsupported action: ${action}`);
  return entry.amount.toString();
}

export function buildLivingPaymentMessage(tokenId, action, route) {
  if (!tokenId) throw new Error("Token ID is required");
  if (!ACTIONS[action]) throw new Error(`Unsupported action: ${action}`);
  if (!["sponsor", "burn"].includes(route)) {
    throw new Error(`Unsupported economic route: ${route}`);
  }
  return JSON.stringify({ token_id: String(tokenId), action, route });
}

export function formatNuve(atomic) {
  const raw = BigInt(atomic || "0");
  const whole = raw / NUVE_UNIT;
  const fraction = (raw % NUVE_UNIT).toString().padStart(NUVE_DECIMALS, "0");
  return `${whole}.${fraction}`;
}

export function parseNuve(value) {
  const raw = String(value ?? "").trim();
  if (!/^\d+(?:\.\d{0,4})?$/.test(raw)) {
    throw new Error("NUVE amount must have at most 4 decimal places");
  }
  const [whole, fraction = ""] = raw.split(".");
  const atomic =
    BigInt(whole) * NUVE_UNIT +
    BigInt((fraction + "0".repeat(NUVE_DECIMALS)).slice(0, NUVE_DECIMALS));
  if (atomic <= 0n) throw new Error("NUVE amount must be positive");
  return atomic.toString();
}

export function parseTokenExtra(token) {
  const raw = token?.metadata?.extra;
  if (!raw) return {};
  try {
    const value = typeof raw === "string" ? JSON.parse(raw) : raw;
    return value && typeof value === "object" && !Array.isArray(value) ? value : {};
  } catch {
    return {};
  }
}

export function nsToMs(value) {
  return Number(BigInt(value || 0) / 1_000_000n);
}

export function actionReady(state, nowMs = Date.now()) {
  if (!state?.active_action || !state?.ready_at_ns) return false;
  return nowMs >= nsToMs(state.ready_at_ns);
}

function assertConfigured() {
  if (!contractsConfigured) {
    throw new Error("Hackathon Testnet contracts are not configured yet");
  }
}

function zeroCodeHash(value) {
  return !value || value === "11111111111111111111111111111111";
}

async function settledValue(promise) {
  try {
    return { ok: true, value: await promise };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : String(error) };
  }
}

export async function loadAccountState(accountId) {
  assertConfigured();
  if (!accountId) throw new Error("Wallet account is required");
  const [tokens, nuveBalance, nuveClaimUsed, nftClaimUsed, alignment, gameConfig, nuveStorage] =
    await Promise.all([
      viewFunction(appConfig.nftContractId, "nft_tokens_for_owner", { account_id: accountId, limit: 50 }),
      viewFunction(appConfig.nuveContractId, "ft_balance_of", { account_id: accountId }),
      viewFunction(appConfig.nuveContractId, "has_claimed_starter", { account_id: accountId }),
      viewFunction(appConfig.nftContractId, "starter_claim_used", { account_id: accountId }),
      viewFunction(appConfig.nftContractId, "get_global_alignment"),
      viewFunction(appConfig.nftContractId, "get_living_game_config"),
      viewFunction(appConfig.nuveContractId, "storage_balance_of", { account_id: accountId }),
    ]);
  return {
    tokens: Array.isArray(tokens) ? tokens : [],
    nuveBalance: String(nuveBalance || "0"),
    nuveClaimUsed: Boolean(nuveClaimUsed),
    nftClaimUsed: Boolean(nftClaimUsed),
    alignment,
    gameConfig,
    nuveStorage,
  };
}

export async function loadNuviAccountProof(tokenId, token, living) {
  assertConfigured();
  const id = String(tokenId || "");
  if (!id) return null;
  const [expectedId, creationDeposit] = await Promise.all([
    viewFunction(appConfig.nftContractId, "expected_nuvi_account_id", { token_id: id }),
    viewFunction(appConfig.nftContractId, "nuvi_account_creation_deposit"),
  ]);
  const extra = parseTokenExtra(token);
  const mutationCount = Number(living?.mutation_count || 0);
  const metadataStatus = String(extra.nuvi_account_status || "");
  const accountId = String(
    extra.nuvi_account_id || extra.nuvi_account_expected_id || expectedId || "",
  );
  if (mutationCount === 0) {
    return {
      status: "not_created",
      metadataStatus,
      accountId,
      expectedId: String(expectedId || ""),
      creationDeposit: String(creationDeposit || "0"),
      failurePhase: "",
      verified: false,
      account: null,
      accessKeys: null,
      executor: null,
      nuveBalance: "0",
      nuveStorage: null,
    };
  }
  const [accountResult, keysResult, executorResult, balanceResult, storageResult] =
    await Promise.all([
      settledValue(viewAccount(accountId)),
      settledValue(viewAccessKeyList(accountId)),
      settledValue(viewFunction(accountId, "get_config")),
      settledValue(viewFunction(appConfig.nuveContractId, "ft_balance_of", { account_id: accountId })),
      settledValue(viewFunction(appConfig.nuveContractId, "storage_balance_of", { account_id: accountId })),
    ]);
  const accessKeys = keysResult.ok
    ? Array.isArray(keysResult.value?.keys) ? keysResult.value.keys : []
    : null;
  const executor = executorResult.ok ? executorResult.value : null;
  const account = accountResult.ok ? accountResult.value : null;
  const nuveStorage = storageResult.ok ? storageResult.value : null;
  const nuveBalance = balanceResult.ok ? String(balanceResult.value || "0") : "0";
  const executorMatches =
    executor?.nft_contract_id === appConfig.nftContractId &&
    String(executor?.token_id || "") === id &&
    executor?.nuve_ft_id === appConfig.nuveContractId;
  const verified =
    Boolean(account) &&
    !zeroCodeHash(account?.code_hash) &&
    Array.isArray(accessKeys) &&
    accessKeys.length === 0 &&
    executorMatches &&
    Boolean(nuveStorage) &&
    extra.nuvi_account_id === accountId;
  let status = metadataStatus || "provisioning";
  if (status === "active" && !verified) status = "failed";
  if (verified) status = "active";
  return {
    status,
    metadataStatus,
    accountId,
    expectedId: String(expectedId || ""),
    creationDeposit: String(creationDeposit || "0"),
    failurePhase: String(extra.nuvi_account_failure_phase || ""),
    verified,
    account,
    accessKeys,
    executor,
    nuveBalance,
    nuveStorage,
  };
}

export async function loadTokenState(tokenId) {
  assertConfigured();
  if (!tokenId) return null;
  const [token, living, mutation] = await Promise.all([
    viewFunction(appConfig.nftContractId, "nft_token", { token_id: String(tokenId) }),
    viewFunction(appConfig.nftContractId, "get_living_state", { token_id: String(tokenId) }),
    viewFunction(appConfig.nftContractId, "get_mutation_preview", { token_id: String(tokenId) }),
  ]);
  const nuviAccount = await loadNuviAccountProof(tokenId, token, living);
  return { token, living, mutation, nuviAccount };
}

export async function claimStarterNuvi(connector) {
  assertConfigured();
  return signFunctionCall(connector, {
    receiverId: appConfig.nftContractId,
    methodName: "claim_starter_nuvi",
    gasTgas: 80,
  });
}

export async function ensureNuveStorage(connector, accountId) {
  assertConfigured();
  const existing = await viewFunction(appConfig.nuveContractId, "storage_balance_of", { account_id: accountId });
  if (existing) return null;
  const bounds = await viewFunction(appConfig.nuveContractId, "storage_balance_bounds");
  const minimum = bounds?.min;
  if (!minimum) throw new Error("Unable to read NUVE storage minimum");
  return signFunctionCall(connector, {
    receiverId: appConfig.nuveContractId,
    methodName: "storage_deposit",
    args: { account_id: accountId, registration_only: true },
    gasTgas: 40,
    deposit: minimum,
  });
}

export async function claimStarterNuve(connector, accountId) {
  assertConfigured();
  const results = [];
  const storageTx = await ensureNuveStorage(connector, accountId);
  if (storageTx) results.push(storageTx);
  results.push(await signFunctionCall(connector, {
    receiverId: appConfig.nuveContractId,
    methodName: "starter_claim",
    gasTgas: 50,
  }));
  return { results };
}

export async function startLivingAction(connector, { tokenId, action, route }) {
  assertConfigured();
  return signFunctionCall(connector, {
    receiverId: appConfig.nuveContractId,
    methodName: "ft_transfer_call",
    args: {
      receiver_id: appConfig.nftContractId,
      amount: actionAmountAtomic(action),
      memo: `NUVI Living ${action} via ${route}`,
      msg: buildLivingPaymentMessage(tokenId, action, route),
    },
    gasTgas: 220,
    deposit: "1",
  });
}

export async function completeLivingAction(connector, tokenId) {
  assertConfigured();
  return signFunctionCall(connector, {
    receiverId: appConfig.nftContractId,
    methodName: "complete_living_action",
    args: { token_id: String(tokenId) },
    gasTgas: 120,
  });
}

export async function mutateNuvi(connector, tokenId) {
  assertConfigured();
  const deposit = await viewFunction(appConfig.nftContractId, "nuvi_account_creation_deposit");
  if (!deposit) throw new Error("Unable to read NUVI Account provisioning deposit");
  return signFunctionCall(connector, {
    receiverId: appConfig.nftContractId,
    methodName: "mutate",
    args: { token_id: String(tokenId) },
    gasTgas: 220,
    deposit: String(deposit),
  });
}

export async function sendNuviAccountNuve(
  connector,
  { nuviAccountId, receiverId, amountNuve },
) {
  assertConfigured();
  if (!nuviAccountId) throw new Error("NUVI Account is not active");
  if (!receiverId) throw new Error("Recipient is required");
  const storage = await viewFunction(appConfig.nuveContractId, "storage_balance_of", {
    account_id: receiverId,
  });
  if (!storage) throw new Error("Recipient is not registered for NUVE");
  return signFunctionCall(connector, {
    receiverId: nuviAccountId,
    methodName: "request_ft_transfer",
    args: { receiver_id: receiverId, amount: parseNuve(amountNuve) },
    gasTgas: 120,
  });
}
