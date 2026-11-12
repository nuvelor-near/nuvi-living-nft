import { appConfig, contractsConfigured } from "../config";
import { viewFunction } from "./rpc";
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

  return JSON.stringify({
    token_id: String(tokenId),
    action,
    route,
  });
}

export function formatNuve(atomic) {
  const raw = BigInt(atomic || "0");
  const whole = raw / NUVE_UNIT;
  const fraction = (raw % NUVE_UNIT).toString().padStart(NUVE_DECIMALS, "0");
  return `${whole}.${fraction}`;
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

export async function loadAccountState(accountId) {
  assertConfigured();
  if (!accountId) throw new Error("Wallet account is required");

  const [
    tokens,
    nuveBalance,
    nuveClaimUsed,
    nftClaimUsed,
    alignment,
    gameConfig,
    nuveStorage,
  ] = await Promise.all([
    viewFunction(appConfig.nftContractId, "nft_tokens_for_owner", {
      account_id: accountId,
      limit: 50,
    }),
    viewFunction(appConfig.nuveContractId, "ft_balance_of", {
      account_id: accountId,
    }),
    viewFunction(appConfig.nuveContractId, "has_claimed_starter", {
      account_id: accountId,
    }),
    viewFunction(appConfig.nftContractId, "starter_claim_used", {
      account_id: accountId,
    }),
    viewFunction(appConfig.nftContractId, "get_global_alignment"),
    viewFunction(appConfig.nftContractId, "get_living_game_config"),
    viewFunction(appConfig.nuveContractId, "storage_balance_of", {
      account_id: accountId,
    }),
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

export async function loadTokenState(tokenId) {
  assertConfigured();
  if (!tokenId) return null;

  const [token, living, mutation] = await Promise.all([
    viewFunction(appConfig.nftContractId, "nft_token", {
      token_id: String(tokenId),
    }),
    viewFunction(appConfig.nftContractId, "get_living_state", {
      token_id: String(tokenId),
    }),
    viewFunction(appConfig.nftContractId, "get_mutation_preview", {
      token_id: String(tokenId),
    }),
  ]);

  return { token, living, mutation };
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
  const existing = await viewFunction(
    appConfig.nuveContractId,
    "storage_balance_of",
    { account_id: accountId },
  );
  if (existing) return null;

  const bounds = await viewFunction(
    appConfig.nuveContractId,
    "storage_balance_bounds",
  );
  const minimum = bounds?.min;
  if (!minimum) throw new Error("Unable to read NUVE storage minimum");

  return signFunctionCall(connector, {
    receiverId: appConfig.nuveContractId,
    methodName: "storage_deposit",
    args: {
      account_id: accountId,
      registration_only: true,
    },
    gasTgas: 40,
    deposit: minimum,
  });
}

export async function claimStarterNuve(connector, accountId) {
  assertConfigured();
  const results = [];

  const storageTx = await ensureNuveStorage(connector, accountId);
  if (storageTx) results.push(storageTx);

  results.push(
    await signFunctionCall(connector, {
      receiverId: appConfig.nuveContractId,
      methodName: "starter_claim",
      gasTgas: 50,
    }),
  );

  return { results };
}

export async function startLivingAction(
  connector,
  { tokenId, action, route },
) {
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
  return signFunctionCall(connector, {
    receiverId: appConfig.nftContractId,
    methodName: "mutate",
    args: { token_id: String(tokenId) },
    gasTgas: 80,
  });
}
