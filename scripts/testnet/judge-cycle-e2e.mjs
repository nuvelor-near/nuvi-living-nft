import { connect, keyStores, KeyPair } from "near-api-js";
import { readFile, writeFile, chmod } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";

const NETWORK = "testnet";
const RPC = "https://rpc.testnet.fastnear.com";
const SPONSOR_URL = "https://nuvelor.gg/api/hackathon/nuvi/testnet-fund";
const PARENT_ID = "nvlborc70517034832.testnet";
const JUDGE_ID = process.env.JUDGE_ID || "judgecycle-1114a.nvlborc70517034832.testnet";
const TGAS = 1_000_000_000_000n;
const ONE_NEAR = 10n ** 24n;
const NUVE_UNIT = 10_000n;

const dep = JSON.parse(await readFile("../../deployments/testnet.json", "utf8"));
const NFT = dep.living_nft_account_id;
const FT = dep.nuve_ft_account_id;
const credDir = path.join(homedir(), ".near-credentials", NETWORK);
const parentCred = JSON.parse(await readFile(path.join(credDir, PARENT_ID + ".json"), "utf8"));

const ks = new keyStores.InMemoryKeyStore();
await ks.setKey(NETWORK, PARENT_ID, KeyPair.fromString(parentCred.private_key));
const near = await connect({ networkId: NETWORK, nodeUrl: RPC, keyStore: ks });
const provider = near.connection.provider;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function accountState(id) {
  try {
    return await provider.query({ request_type: "view_account", finality: "final", account_id: id });
  } catch {
    return null;
  }
}
async function view(account, contractId, methodName, args = {}) {
  return account.viewFunction({ contractId, methodName, args, blockQuery: { finality: "final" } });
}
async function poll(read, predicate, label, timeoutMs = 90000) {
  const start = Date.now();
  let last;
  while (Date.now() - start < timeoutMs) {
    last = await read();
    if (predicate(last)) return last;
    await sleep(750);
  }
  throw new Error("timeout waiting for " + label + ": " + JSON.stringify(last));
}
function hash(out) {
  return out?.transaction?.hash || out?.transaction_outcome?.id || null;
}
async function call(account, methodName, contractId, args = {}, gasTgas = 100, deposit = 0n) {
  const out = await account.functionCall({
    contractId,
    methodName,
    args,
    gas: BigInt(gasTgas) * TGAS,
    attachedDeposit: deposit,
  });
  evidence.transactions.push({ method: methodName, contract_id: contractId, tx_hash: hash(out) });
  return out;
}

const evidence = {
  schema: "nuvi-judge-cycle-e2e-v1",
  generated_at_utc: new Date().toISOString(),
  account_id: JUDGE_ID,
  sponsor_url: SPONSOR_URL,
  transactions: [],
};

if (await accountState(JUDGE_ID)) {
  throw new Error("Judge E2E account already exists; choose a fresh JUDGE_ID");
}

const kp = KeyPair.fromRandom("ed25519");
const credPath = path.join(credDir, JUDGE_ID + ".json");
await writeFile(
  credPath,
  JSON.stringify(
    { account_id: JUDGE_ID, public_key: kp.getPublicKey().toString(), private_key: kp.toString() },
    null,
    2,
  ) + "\n",
  { flag: "wx", mode: 0o600 },
);
await chmod(credPath, 0o600);

const parent = await near.account(PARENT_ID);
const createOut = await parent.createAccount(JUDGE_ID, kp.getPublicKey(), 50_000_000_000_000_000_000_000n);
evidence.transactions.push({ method: "create_account", tx_hash: hash(createOut), amount_yocto: "50000000000000000000000" });
await ks.setKey(NETWORK, JUDGE_ID, kp);
const judge = await near.account(JUDGE_ID);
await poll(() => accountState(JUDGE_ID), Boolean, "fresh judge account");

const sponsorResponse = await fetch(SPONSOR_URL, {
  method: "POST",
  headers: { "Content-Type": "application/json", Origin: "https://nuvelor.gg" },
  body: JSON.stringify({ account_id: JUDGE_ID }),
});
const sponsorBody = await sponsorResponse.json();
if (!sponsorResponse.ok || sponsorBody.status !== "funded") {
  throw new Error("Sponsor failed: " + JSON.stringify(sponsorBody));
}
evidence.sponsor = sponsorBody;
await poll(
  async () => BigInt((await accountState(JUDGE_ID)).amount),
  (v) => v >= 2n * ONE_NEAR,
  "2 NEAR sponsor balance",
);
evidence.balance_after_sponsor_yocto = (await accountState(JUDGE_ID)).amount;

const bounds = await view(judge, FT, "storage_balance_bounds");
await call(
  judge,
  "storage_deposit",
  FT,
  { account_id: JUDGE_ID, registration_only: true },
  40,
  BigInt(bounds.min),
);
await call(judge, "claim_starter_nuvi", NFT, {}, 80, 0n);
await call(judge, "starter_claim", FT, {}, 50, 0n);

const owned = await poll(
  () => view(judge, NFT, "nft_tokens_for_owner", { account_id: JUDGE_ID, limit: 50 }),
  (x) => Array.isArray(x) && x.length > 0,
  "Starter NUVI",
);
const tokenId = String(owned[0].token_id);
evidence.token_id = tokenId;

await call(
  judge,
  "ft_transfer_call",
  FT,
  {
    receiver_id: NFT,
    amount: (10n * NUVE_UNIT).toString(),
    memo: "Hackathon judge E2E work_a",
    msg: JSON.stringify({ token_id: tokenId, action: "work_a", route: "sponsor" }),
  },
  220,
  1n,
);

const living = await poll(
  () => view(judge, NFT, "get_living_state", { token_id: tokenId }),
  (x) => Number(x?.xp || 0) >= 1 && x?.active_action == null,
  "instant Work XP",
);
evidence.living_after_work = living;

const preview = await view(judge, NFT, "get_mutation_preview", { token_id: tokenId });
if (!preview?.eligible || Number(preview.threshold_xp) !== 1) {
  throw new Error("Mutation not eligible after one Work: " + JSON.stringify(preview));
}
evidence.mutation_preview = preview;

const provisioningDeposit = BigInt(await view(judge, NFT, "nuvi_account_creation_deposit"));
evidence.provisioning_deposit_yocto = provisioningDeposit.toString();
await call(judge, "mutate", NFT, { token_id: tokenId }, 220, provisioningDeposit);

const expectedChild = String(await view(judge, NFT, "expected_nuvi_account_id", { token_id: tokenId }));
evidence.child_account_id = expectedChild;
const activeToken = await poll(
  () => view(judge, NFT, "nft_token", { token_id: tokenId }),
  (t) => {
    try {
      const extra = JSON.parse(t?.metadata?.extra || "{}");
      return extra.nuvi_account_status === "active" && extra.nuvi_account_id === expectedChild;
    } catch {
      return false;
    }
  },
  "NUVI Account active",
);
evidence.final_token = activeToken;

const keys = await provider.query({
  request_type: "view_access_key_list",
  finality: "final",
  account_id: expectedChild,
});
if ((keys.keys || []).length !== 0) throw new Error("NUVI Account unexpectedly has access keys");
evidence.child_access_key_count = 0;

const childBefore = BigInt(await view(judge, FT, "ft_balance_of", { account_id: expectedChild }));
if (childBefore < NUVE_UNIT) throw new Error("NUVI Account balance too low");
evidence.child_nuve_before = childBefore.toString();

await call(
  judge,
  "request_ft_transfer",
  expectedChild,
  { receiver_id: JUDGE_ID, amount: NUVE_UNIT.toString() },
  120,
  0n,
);

const childAfter = await poll(
  async () => BigInt(await view(judge, FT, "ft_balance_of", { account_id: expectedChild })),
  (v) => v === childBefore - NUVE_UNIT,
  "NUVI Account transfer",
);
evidence.child_nuve_after = childAfter.toString();
evidence.final_balance_yocto = (await accountState(JUDGE_ID)).amount;
evidence.status = "PASS";
evidence.generated_at_utc = new Date().toISOString();

await writeFile("../../deployments/judge-cycle-e2e-testnet.json", JSON.stringify(evidence, null, 2) + "\n", "utf8");
console.log(JSON.stringify({
  status: evidence.status,
  account_id: JUDGE_ID,
  sponsor_tx: evidence.sponsor.tx_hash,
  token_id: tokenId,
  child_account_id: expectedChild,
  child_access_key_count: 0,
  final_balance_yocto: evidence.final_balance_yocto,
  transactions: evidence.transactions,
}, null, 2));
