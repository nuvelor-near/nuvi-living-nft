import { connect, keyStores, KeyPair } from "near-api-js";
import { readFile, writeFile, chmod } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";

const NETWORK_ID = "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const TGAS = 1_000_000_000_000n;
const ONE_NEAR = 10n ** 24n;
const NUVE_UNIT = 10_000n;
const PROBE_ID = process.env.PROBE_ID || "judgeprobe-1114a.nvlborc70517034832.testnet";
const INITIAL_NEAR = process.env.PROBE_INITIAL_NEAR || "2.5";
const TOPUP_SOURCE = process.env.PROBE_TOPUP_SOURCE || "nvlownrc70517034832.testnet";
const REPORT_PATH = process.env.REPORT_PATH || "../../deployments/fresh-user-budget-testnet.json";
const PARENT_MIN_NEAR = "3.0";

const dep = JSON.parse(await readFile("../../deployments/testnet.json", "utf8"));
const PARENT_ID = dep.parent_account_id;
const LIVING_ID = dep.living_nft_account_id;
const NUVE_ID = dep.nuve_ft_account_id;

function yocto(value) {
  const [w, f = ""] = String(value).split(".");
  return BigInt(w) * ONE_NEAR + BigInt((f + "0".repeat(24)).slice(0, 24));
}
function near(value) {
  const n = BigInt(value);
  const whole = n / ONE_NEAR;
  const frac = (n % ONE_NEAR).toString().padStart(24, "0").slice(0, 6);
  return `${whole}.${frac}`;
}
async function loadCredential(accountId) {
  const p = path.join(homedir(), ".near-credentials", NETWORK_ID, accountId + ".json");
  return JSON.parse(await readFile(p, "utf8"));
}
async function state(provider, accountId) {
  try {
    return await provider.query({ request_type: "view_account", finality: "final", account_id: accountId });
  } catch (e) {
    const m = String(e?.message || e);
    if (m.includes("UNKNOWN_ACCOUNT") || m.includes("does not exist") || m.includes("doesn't exist")) return null;
    throw e;
  }
}
async function view(account, contractId, methodName, args = {}) {
  return account.viewFunction({ contractId, methodName, args, blockQuery: { finality: "final" } });
}
function txHash(outcome) {
  return outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function poll(read, predicate, label, timeoutMs = 60000) {
  const started = Date.now(); let last;
  while (Date.now() - started < timeoutMs) {
    last = await read();
    if (predicate(last)) return last;
    await sleep(1000);
  }
  throw new Error("timeout " + label + " " + JSON.stringify(last));
}

const parentCred = await loadCredential(PARENT_ID);
const topupCred = await loadCredential(TOPUP_SOURCE);
const keyStore = new keyStores.InMemoryKeyStore();
await keyStore.setKey(NETWORK_ID, PARENT_ID, KeyPair.fromString(parentCred.private_key));
await keyStore.setKey(NETWORK_ID, TOPUP_SOURCE, KeyPair.fromString(topupCred.private_key));
const nearConn = await connect({ networkId: NETWORK_ID, nodeUrl: RPC_URL, keyStore });
const provider = nearConn.connection.provider;
const parent = await nearConn.account(PARENT_ID);

let probeState = await state(provider, PROBE_ID);
if (!probeState) {
  const parentState = await state(provider, PARENT_ID);
  if (BigInt(parentState.amount) < yocto(PARENT_MIN_NEAR)) {
    const need = yocto(PARENT_MIN_NEAR) - BigInt(parentState.amount);
    const topup = await nearConn.account(TOPUP_SOURCE);
    const o = await topup.sendMoney(PARENT_ID, need);
    console.log("parent_topup_tx=" + txHash(o) + " amount_near=" + near(need));
  }
  const kp = KeyPair.fromRandom("ed25519");
  const credPath = path.join(homedir(), ".near-credentials", NETWORK_ID, PROBE_ID + ".json");
  const c = { account_id: PROBE_ID, public_key: kp.getPublicKey().toString(), private_key: kp.toString() };
  await writeFile(credPath, JSON.stringify(c, null, 2) + "\n", { mode: 0o600, flag: "wx" });
  await chmod(credPath, 0o600);
  const o = await parent.createAccount(PROBE_ID, kp.getPublicKey(), yocto(INITIAL_NEAR));
  console.log("probe_create_tx=" + txHash(o) + " initial_near=" + INITIAL_NEAR);
  await keyStore.setKey(NETWORK_ID, PROBE_ID, kp);
} else {
  const c = await loadCredential(PROBE_ID);
  await keyStore.setKey(NETWORK_ID, PROBE_ID, KeyPair.fromString(c.private_key));
}
const probe = await nearConn.account(PROBE_ID);
probeState = await poll(() => state(provider, PROBE_ID), Boolean, "probe account final state", 45000);
const rows = [];
let firstAmount = BigInt(probeState.amount);
const initialNftClaim = await view(probe, LIVING_ID, "starter_claim_used", { account_id: PROBE_ID });
const initialNuveClaim = await view(probe, NUVE_ID, "has_claimed_starter", { account_id: PROBE_ID });
const initialStorage = await view(probe, NUVE_ID, "storage_balance_of", { account_id: PROBE_ID });
console.log("initial_checks=" + JSON.stringify({ initialNftClaim, initialNuveClaim, initialStorage }));
if (initialNftClaim || initialNuveClaim || initialStorage) {
  throw new Error("Probe account is not a clean cold-start account; choose a new PROBE_ID");
}

async function callStep(name, { contractId, methodName, args = {}, gasTgas = 100, deposit = 0n }) {
  const before = BigInt((await state(provider, PROBE_ID)).amount);
  const out = await probe.functionCall({
    contractId, methodName, args,
    gas: BigInt(gasTgas) * TGAS,
    attachedDeposit: deposit,
  });
  await sleep(1200);
  const after = BigInt((await state(provider, PROBE_ID)).amount);
  const row = {
    step: name,
    tx_hash: txHash(out),
    before_yocto: before.toString(),
    after_yocto: after.toString(),
    delta_yocto: (before - after).toString(),
    delta_near: near(before - after),
    attached_deposit_yocto: deposit.toString(),
    gas_tgas: gasTgas,
  };
  rows.push(row);
  console.log(JSON.stringify(row));
  return out;
}

const bounds = await view(probe, NUVE_ID, "storage_balance_bounds");
await callStep("nuve_storage", {
  contractId: NUVE_ID, methodName: "storage_deposit",
  args: { account_id: PROBE_ID, registration_only: true }, gasTgas: 40,
  deposit: BigInt(bounds.min),
});
await callStep("claim_starter_nuvi", { contractId: LIVING_ID, methodName: "claim_starter_nuvi", gasTgas: 80 });
await callStep("claim_starter_nuve", { contractId: NUVE_ID, methodName: "starter_claim", gasTgas: 50 });

const owned = await view(probe, LIVING_ID, "nft_tokens_for_owner", { account_id: PROBE_ID, limit: 50 });
const tokenId = String(owned[0].token_id);
console.log("probe_token_id=" + tokenId);

async function living() { return view(probe, LIVING_ID, "get_living_state", { token_id: tokenId }); }
async function runAction(action, route, amount) {
  await callStep("start_" + action + "_" + route, {
    contractId: NUVE_ID, methodName: "ft_transfer_call",
    args: {
      receiver_id: LIVING_ID, amount: String(amount), memo: "Fresh judge budget probe",
      msg: JSON.stringify({ token_id: tokenId, action, route }),
    },
    gasTgas: 220, deposit: 1n,
  });
  const active = await poll(living, (x) => x?.active_action === action, "active " + action);
  const readyMs = Number(BigInt(active.ready_at_ns || "0") / 1_000_000n);
  const waitMs = Math.max(0, readyMs - Date.now()) + 1300;
  if (waitMs > 0) await sleep(waitMs);
  await callStep("complete_" + action, {
    contractId: LIVING_ID, methodName: "complete_living_action",
    args: { token_id: tokenId }, gasTgas: 120,
  });
  await poll(living, (x) => !x?.active_action, "complete " + action);
}

await runAction("feed", "sponsor", 5n * NUVE_UNIT);
for (const [a, r] of [["work_a","sponsor"],["work_a","sponsor"],["work_b","burn"],["work_b","burn"],["work_a","sponsor"]]) {
  await runAction(a, r, 10n * NUVE_UNIT);
}

const preview = await view(probe, LIVING_ID, "get_mutation_preview", { token_id: tokenId });
if (!preview?.eligible) throw new Error("mutation not eligible");
const provisioningDeposit = BigInt(await view(probe, LIVING_ID, "nuvi_account_creation_deposit"));
await callStep("mutate_and_provision", {
  contractId: LIVING_ID, methodName: "mutate",
  args: { token_id: tokenId }, gasTgas: 220, deposit: provisioningDeposit,
});

const expectedChild = String(await view(probe, LIVING_ID, "expected_nuvi_account_id", { token_id: tokenId }));
const childToken = await poll(
  () => view(probe, LIVING_ID, "nft_token", { token_id: tokenId }),
  (t) => {
    try { const e = JSON.parse(t?.metadata?.extra || "{}"); return e.nuvi_account_status === "active" && e.nuvi_account_id === expectedChild; }
    catch { return false; }
  }, "child active", 90000,
);
console.log("child_active=" + expectedChild);
const childBalance = await view(probe, NUVE_ID, "ft_balance_of", { account_id: expectedChild });
console.log("child_nuve_before=" + childBalance);
await callStep("smart_account_transfer_1_nuve", {
  contractId: expectedChild, methodName: "request_ft_transfer",
  args: { receiver_id: PROBE_ID, amount: NUVE_UNIT.toString() }, gasTgas: 120,
});
const finalAmount = BigInt((await state(provider, PROBE_ID)).amount);
const report = {
  schema: "nuvi-fresh-user-budget-v1",
  measured_at_utc: new Date().toISOString(),
  account_id: PROBE_ID,
  token_id: tokenId,
  child_account_id: expectedChild,
  initial_balance_yocto: firstAmount.toString(),
  initial_balance_near: near(firstAmount),
  final_balance_yocto: finalAmount.toString(),
  final_balance_near: near(finalAmount),
  total_spent_yocto: (firstAmount - finalAmount).toString(),
  total_spent_near: near(firstAmount - finalAmount),
  provisioning_deposit_yocto: provisioningDeposit.toString(),
  provisioning_deposit_near: near(provisioningDeposit),
  steps: rows,
  child_nuve_before: String(childBalance),
};
await writeFile(REPORT_PATH, JSON.stringify(report, null, 2) + "\n", "utf8");
console.log("REPORT=" + JSON.stringify(report));
