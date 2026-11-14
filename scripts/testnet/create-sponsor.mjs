import { connect, keyStores, KeyPair } from "near-api-js";
import { readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";

const NETWORK_ID = "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const PARENT_ID = "nvlborc70517034832.testnet";
const SPONSOR_ID = "sponsor-hack26.nvlborc70517034832.testnet";
const SPONSOR_PUBLIC_KEY = "ed25519:2CRoethPEW79nbnQr9N2LEx8o2w8GpUFp9bL2fXZtjR8";
const TARGET = 12_200_000_000_000_000_000_000_000n;
const CREATE_BALANCE = 200_000_000_000_000_000_000_000n;
const SOURCE_RESERVE = 500_000_000_000_000_000_000_000n;
const MAX_FROM_SOURCE = 4_000_000_000_000_000_000_000_000n;
const FUNDERS = [
  "nvlr-market-fund-0929.testnet",
  "mktv2rcfaucet0929.testnet",
  "nvlalrc70517034832.testnet",
];

const credentialsDir = path.join(homedir(), ".near-credentials", NETWORK_ID);
async function loadCredential(accountId) {
  const value = JSON.parse(
    await readFile(path.join(credentialsDir, accountId + ".json"), "utf8"),
  );
  if (value.account_id !== accountId || !value.private_key) {
    throw new Error("Credential missing or mismatched for " + accountId);
  }
  return value;
}
async function state(provider, accountId) {
  try {
    return await provider.query({
      request_type: "view_account",
      finality: "final",
      account_id: accountId,
    });
  } catch (error) {
    const message = String(error?.message || error);
    if (message.includes("UNKNOWN_ACCOUNT") || message.includes("does not exist") || message.includes("doesn't exist")) {
      return null;
    }
    throw error;
  }
}
function txHash(outcome) {
  return outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
}

const keyStore = new keyStores.InMemoryKeyStore();
for (const id of [PARENT_ID, ...FUNDERS]) {
  const credential = await loadCredential(id);
  await keyStore.setKey(
    NETWORK_ID,
    id,
    KeyPair.fromString(credential.private_key),
  );
}
const near = await connect({
  networkId: NETWORK_ID,
  nodeUrl: RPC_URL,
  keyStore,
});
const provider = near.connection.provider;
const transactions = [];

let sponsor = await state(provider, SPONSOR_ID);
if (!sponsor) {
  const parent = await near.account(PARENT_ID);
  const outcome = await parent.createAccount(
    SPONSOR_ID,
    SPONSOR_PUBLIC_KEY,
    CREATE_BALANCE,
  );
  transactions.push({
    step: "create_sponsor",
    tx_hash: txHash(outcome),
    amount_yocto: CREATE_BALANCE.toString(),
  });
  sponsor = await state(provider, SPONSOR_ID);
}

const keys = await provider.query({
  request_type: "view_access_key_list",
  finality: "final",
  account_id: SPONSOR_ID,
});
const publicKeys = (keys.keys || []).map((item) => item.public_key);
if (
  publicKeys.length !== 1 ||
  publicKeys[0] !== SPONSOR_PUBLIC_KEY
) {
  throw new Error(
    "Sponsor access key mismatch: " + JSON.stringify(publicKeys),
  );
}

let sponsorAmount = BigInt(sponsor.amount);
for (const funderId of FUNDERS) {
  if (sponsorAmount >= TARGET) break;
  const funderState = await state(provider, funderId);
  const funderAmount = BigInt(funderState?.amount || "0");
  const available =
    funderAmount > SOURCE_RESERVE ? funderAmount - SOURCE_RESERVE : 0n;
  const required = TARGET - sponsorAmount;
  const amount = [available, required, MAX_FROM_SOURCE].reduce(
    (smallest, value) => (value < smallest ? value : smallest),
  );
  if (amount <= 0n) continue;
  const funder = await near.account(funderId);
  const outcome = await funder.sendMoney(SPONSOR_ID, amount);
  transactions.push({
    step: "fund_sponsor",
    from: funderId,
    amount_yocto: amount.toString(),
    tx_hash: txHash(outcome),
  });
  sponsor = await state(provider, SPONSOR_ID);
  sponsorAmount = BigInt(sponsor.amount);
}
if (sponsorAmount < TARGET) {
  throw new Error("Sponsor funding target not reached");
}

const evidence = {
  schema: "nuvi-testnet-sponsor-funding-v1",
  generated_at_utc: new Date().toISOString(),
  network_id: NETWORK_ID,
  sponsor_account_id: SPONSOR_ID,
  sponsor_public_key: SPONSOR_PUBLIC_KEY,
  target_balance_yocto: TARGET.toString(),
  final_balance_yocto: sponsorAmount.toString(),
  transactions,
};
await writeFile(
  "../../deployments/testnet-sponsor.json",
  JSON.stringify(evidence, null, 2) + "\n",
  "utf8",
);
console.log(JSON.stringify(evidence));
