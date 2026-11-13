import { connect, keyStores, KeyPair } from "near-api-js";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NETWORK_ID = "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const TGAS = 1_000_000_000_000n;
const NUVE_UNIT = 10_000n;

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const deployment = JSON.parse(
  await readFile(path.join(repoRoot, "deployments/testnet.json"), "utf8"),
);

const LIVING_ID = deployment.living_nft_account_id;
const NUVE_ID = deployment.nuve_ft_account_id;
const TREASURY_ID = deployment.treasury_account_id;
const PLAYER_ID = deployment.e2e_player_account_id;
const credentialPath = path.join(
  homedir(),
  ".near-credentials",
  NETWORK_ID,
  `${PLAYER_ID}.json`,
);

const credential = JSON.parse(await readFile(credentialPath, "utf8"));
if (
  credential.account_id !== PLAYER_ID ||
  !credential.private_key ||
  !credential.public_key
) {
  throw new Error("Player credential is missing or mismatched");
}

const keyStore = new keyStores.InMemoryKeyStore();
await keyStore.setKey(
  NETWORK_ID,
  PLAYER_ID,
  KeyPair.fromString(credential.private_key),
);

const near = await connect({
  networkId: NETWORK_ID,
  nodeUrl: RPC_URL,
  keyStore,
});
const player = await near.account(PLAYER_ID);

const transactions = [];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function pollUntil(read, predicate, label, timeoutMs = 20_000) {
  const started = Date.now();
  let last;
  while (Date.now() - started < timeoutMs) {
    last = await read();
    if (predicate(last)) return last;
    await sleep(1_000);
  }
  throw new Error(`Timed out waiting for ${label}: ${JSON.stringify(last)}`);
}

function txHash(outcome) {
  return outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
}

async function view(contractId, methodName, args = {}) {
  return player.viewFunction({
    contractId,
    methodName,
    args,
    blockQuery: { finality: "final" },
  });
}

async function call({
  contractId,
  methodName,
  args = {},
  gasTgas = 100,
  attachedDeposit = 0n,
  evidence = {},
}) {
  const outcome = await player.functionCall({
    contractId,
    methodName,
    args,
    gas: BigInt(gasTgas) * TGAS,
    attachedDeposit,
  });
  const hash = txHash(outcome);
  transactions.push({
    method: methodName,
    contract_id: contractId,
    tx_hash: hash,
    ...evidence,
  });
  console.log("tx=" + methodName + ":" + hash);
  return outcome;
}

async function livingState(tokenId) {
  return view(LIVING_ID, "get_living_state", { token_id: String(tokenId) });
}

async function tokenView(tokenId) {
  return view(LIVING_ID, "nft_token", { token_id: String(tokenId) });
}

async function mutationPreview(tokenId) {
  return view(LIVING_ID, "get_mutation_preview", { token_id: String(tokenId) });
}

async function waitAndComplete(tokenId) {
  let state = await livingState(tokenId);
  if (!state?.active_action) return state;

  const readyMs = Number(BigInt(state.ready_at_ns || "0") / 1_000_000n);
  const delay = Math.max(0, readyMs - Date.now()) + 1_500;
  if (delay > 0) {
    console.log(
      `waiting_action=${state.active_action} delay_ms=${Math.ceil(delay)}`,
    );
    await sleep(delay);
  }

  await call({
    contractId: LIVING_ID,
    methodName: "complete_living_action",
    args: { token_id: String(tokenId) },
    gasTgas: 150,
    evidence: { token_id: String(tokenId), action: state.active_action },
  });

  state = await pollUntil(
    () => livingState(tokenId),
    (value) => !value?.active_action,
    "Living action completion readback",
  );
  return state;
}

async function startAction(tokenId, action, route) {
  const amount =
    action === "feed" ? 5n * NUVE_UNIT : 10n * NUVE_UNIT;

  await call({
    contractId: NUVE_ID,
    methodName: "ft_transfer_call",
    args: {
      receiver_id: LIVING_ID,
      amount: amount.toString(),
      memo: `Hackathon E2E ${action}/${route}`,
      msg: JSON.stringify({
        token_id: String(tokenId),
        action,
        route,
      }),
    },
    gasTgas: 220,
    attachedDeposit: 1n,
    evidence: {
      token_id: String(tokenId),
      action,
      route,
      amount: amount.toString(),
    },
  });

  const state = await pollUntil(
    () => livingState(tokenId),
    (value) => value?.active_action === action,
    `active action ${action}`,
  );
  return state;
}

async function ensureStorage() {
  const existing = await view(NUVE_ID, "storage_balance_of", {
    account_id: PLAYER_ID,
  });
  if (existing) return existing;

  const bounds = await view(NUVE_ID, "storage_balance_bounds");
  await call({
    contractId: NUVE_ID,
    methodName: "storage_deposit",
    args: { account_id: PLAYER_ID, registration_only: true },
    gasTgas: 40,
    attachedDeposit: BigInt(bounds.min),
    evidence: { account_id: PLAYER_ID },
  });

  return view(NUVE_ID, "storage_balance_of", { account_id: PLAYER_ID });
}

await ensureStorage();

let nftClaimed = await view(LIVING_ID, "starter_claim_used", {
  account_id: PLAYER_ID,
});
if (!nftClaimed) {
  await call({
    contractId: LIVING_ID,
    methodName: "claim_starter_nuvi",
    gasTgas: 90,
  });
  nftClaimed = true;
}

let nuveClaimed = await view(NUVE_ID, "has_claimed_starter", {
  account_id: PLAYER_ID,
});
if (!nuveClaimed) {
  await call({
    contractId: NUVE_ID,
    methodName: "starter_claim",
    gasTgas: 60,
  });
  nuveClaimed = true;
}

const owned = await view(LIVING_ID, "nft_tokens_for_owner", {
  account_id: PLAYER_ID,
  limit: 50,
});
if (!Array.isArray(owned) || owned.length === 0) {
  throw new Error("Player has no NUVI after starter claim");
}
const tokenId = String(owned[0].token_id);
console.log("token_id=" + tokenId);

let state = await livingState(tokenId);
if (state.active_action) {
  state = await waitAndComplete(tokenId);
}

// Feed proves the asynchronous NUVE reward path. The spend marker makes this
// step resumable after an interruption.
if (
  Number(state.xp || 0) === 0 &&
  BigInt(state.sponsor_spent || "0") === 0n &&
  BigInt(state.burner_spent || "0") === 0n
) {
  await startAction(tokenId, "feed", "sponsor");
  state = await waitAndComplete(tokenId);
}

// Five Work actions reach the agreed mutation threshold. Route/affinity mix:
// XP 0 -> A/Sponsor
// XP 1 -> A/Sponsor
// XP 2 -> B/Burn
// XP 3 -> B/Burn
// XP 4 -> A/Sponsor
const workPlan = [
  ["work_a", "sponsor"],
  ["work_a", "sponsor"],
  ["work_b", "burn"],
  ["work_b", "burn"],
  ["work_a", "sponsor"],
];

while (Number(state.xp || 0) < 5) {
  const xp = Number(state.xp || 0);
  const [action, route] = workPlan[xp];
  if (!action) throw new Error(`No work plan for XP=${xp}`);

  if (state.active_action) {
    state = await waitAndComplete(tokenId);
    continue;
  }

  await startAction(tokenId, action, route);
  state = await waitAndComplete(tokenId);
}

const previewBeforeMutation = await mutationPreview(tokenId);
if (!state.mutation_count) {
  if (!previewBeforeMutation?.eligible) {
    throw new Error("Mutation should be eligible at XP >= 5");
  }
  const provisioningDeposit = BigInt(
    await view(LIVING_ID, "nuvi_account_creation_deposit"),
  );
  await call({
    contractId: LIVING_ID,
    methodName: "mutate",
    args: { token_id: tokenId },
    gasTgas: 220,
    attachedDeposit: provisioningDeposit,
    evidence: {
      token_id: tokenId,
      provisioning_deposit_yocto: provisioningDeposit.toString(),
      form_a_probability_bps: previewBeforeMutation.form_a_probability_bps,
      form_b_probability_bps: previewBeforeMutation.form_b_probability_bps,
    },
  });
  await pollUntil(
    () => livingState(tokenId),
    (value) => Number(value?.mutation_count || 0) === 1,
    "mutation readback",
  );
}

const finalToken = await tokenView(tokenId);
const finalLiving = await livingState(tokenId);
const finalPreview = await mutationPreview(tokenId);
const alignment = await view(LIVING_ID, "get_global_alignment");
const playerNuveBalance = await view(NUVE_ID, "ft_balance_of", {
  account_id: PLAYER_ID,
});
const treasuryNuveBalance = await view(NUVE_ID, "ft_balance_of", {
  account_id: TREASURY_ID,
});
const totalSupply = await view(NUVE_ID, "ft_total_supply");

if (String(finalToken.token_id) !== tokenId) {
  throw new Error("Token identity changed during mutation");
}
if (Number(finalLiving.xp) < 5) {
  throw new Error("Final XP is below mutation threshold");
}
if (Number(finalLiving.mutation_count) !== 1) {
  throw new Error("Mutation count is not exactly one");
}
if (!["form_a", "form_b"].includes(finalLiving.form_id)) {
  throw new Error(`Unexpected final form: ${finalLiving.form_id}`);
}
if (finalLiving.active_action !== null) {
  throw new Error("Final Living action is still active");
}

const evidence = {
  schema: "nuvi-hackathon-testnet-e2e-v1",
  generated_at_utc: new Date().toISOString(),
  network_id: NETWORK_ID,
  player_account_id: PLAYER_ID,
  living_nft_account_id: LIVING_ID,
  nuve_ft_account_id: NUVE_ID,
  treasury_account_id: TREASURY_ID,
  token_id: tokenId,
  starter_claims: {
    nuvi: nftClaimed,
    nuve: nuveClaimed,
  },
  mutation_preview_before: previewBeforeMutation,
  final_token: finalToken,
  final_living_state: finalLiving,
  final_mutation_preview: finalPreview,
  global_alignment: alignment,
  balances: {
    player_nuve: String(playerNuveBalance),
    treasury_nuve: String(treasuryNuveBalance),
    total_supply: String(totalSupply),
  },
  transactions,
};

const evidencePath = path.join(repoRoot, "deployments/e2e-testnet.json");
await mkdir(path.dirname(evidencePath), { recursive: true });
await writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n", "utf8");

console.log("final_form=" + finalLiving.form_id);
console.log("final_xp=" + finalLiving.xp);
console.log("player_nuve=" + playerNuveBalance);
console.log("treasury_nuve=" + treasuryNuveBalance);
console.log("global_total=" + alignment.global_total);
console.log("evidence=" + path.relative(repoRoot, evidencePath));
