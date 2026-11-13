import { connect, keyStores, KeyPair } from "near-api-js";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NETWORK_ID = process.env.NEAR_NETWORK_ID || "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const EXECUTE = process.env.EXECUTE === "1";
const FUNDING_ACCOUNT = process.env.HACKATHON_FUNDING_ACCOUNT || "";
const STORAGE_PRICE_PER_BYTE = 10n ** 19n;
const RELEASE_RESERVE = 25n * 10n ** 22n;
const EXPECTED_NUVE_STORAGE_MIN = 1_250_000_000_000_000_000_000n;

if (NETWORK_ID !== "testnet") throw new Error("This upgrade script is Testnet-only");

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const deploymentPath = path.join(repoRoot, "deployments/testnet.json");
const evidencePath = path.join(repoRoot, "deployments/smart-account-upgrade-testnet.json");
const deployment = JSON.parse(await readFile(deploymentPath, "utf8"));

if (deployment.network_id !== "testnet") throw new Error("Deployment evidence is not Testnet");

const LIVING_ID = deployment.living_nft_account_id;
const NUVE_ID = deployment.nuve_ft_account_id;
const TREASURY_ID = deployment.treasury_account_id;
const PREVIOUS_LIVING_SHA = deployment.verified?.living_code_sha256;
if (!LIVING_ID || !NUVE_ID || !TREASURY_ID || !PREVIOUS_LIVING_SHA) {
  throw new Error("Deployment evidence is incomplete");
}

const livingWasmPath = path.join(repoRoot, "contracts/living-nft/target/near/nuvelor_nft.wasm");
const embeddedExecutorPath = path.join(repoRoot, "contracts/living-nft/res/nuvi_account.wasm");
const embeddedExecutorShaPath = path.join(repoRoot, "contracts/living-nft/res/nuvi_account.wasm.sha256");
const livingWasm = await readFile(livingWasmPath);
const embeddedExecutor = await readFile(embeddedExecutorPath);
const embeddedSidecar = (await readFile(embeddedExecutorShaPath, "utf8")).trim().split(/\s+/)[0];

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}
function yoctoToNear(value) {
  const raw = BigInt(value);
  return String(raw / 10n ** 24n) + "." + (raw % 10n ** 24n).toString().padStart(24, "0").slice(0, 6);
}
function txHash(outcome) {
  return outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
}
async function loadCredential(accountId) {
  const file = path.join(homedir(), ".near-credentials", NETWORK_ID, accountId + ".json");
  const value = JSON.parse(await readFile(file, "utf8"));
  if (value.account_id !== accountId || !value.private_key || !value.public_key) {
    throw new Error("Credential missing or mismatched for " + accountId);
  }
  return value;
}
async function accountState(provider, accountId) {
  return provider.query({ request_type: "view_account", finality: "final", account_id: accountId });
}
async function codeBytes(provider, accountId) {
  const result = await provider.query({ request_type: "view_code", finality: "final", account_id: accountId });
  return Buffer.from(result.code_base64, "base64");
}
async function codeSha(provider, accountId) {
  return sha256(await codeBytes(provider, accountId));
}
async function view(account, contractId, methodName, args = {}) {
  return account.viewFunction({ contractId, methodName, args, blockQuery: { finality: "final" } });
}
async function waitForAccountAmount(provider, accountId, minimum, timeoutMs = 30_000) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await accountState(provider, accountId);
    if (BigInt(last.amount) >= minimum) return last;
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(
    "Account amount readback stayed below required floor: " +
      (last?.amount || "unknown") +
      " < " +
      minimum,
  );
}
async function waitForCodeSha(provider, accountId, expected, timeoutMs = 45_000) {
  const started = Date.now();
  let last = "";
  while (Date.now() - started < timeoutMs) {
    last = await codeSha(provider, accountId);
    if (last === expected) return last;
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(
    "Living WASM readback stayed at " + last + "; expected " + expected,
  );
}

const keyStore = new keyStores.InMemoryKeyStore();
const livingCredential = await loadCredential(LIVING_ID);
await keyStore.setKey(NETWORK_ID, LIVING_ID, KeyPair.fromString(livingCredential.private_key));
if (FUNDING_ACCOUNT) {
  const fundingCredential = await loadCredential(FUNDING_ACCOUNT);
  await keyStore.setKey(NETWORK_ID, FUNDING_ACCOUNT, KeyPair.fromString(fundingCredential.private_key));
}

const near = await connect({ networkId: NETWORK_ID, nodeUrl: RPC_URL, keyStore });
const provider = near.connection.provider;
const living = await near.account(LIVING_ID);

const targetLivingSha = sha256(livingWasm);
const embeddedExecutorSha = sha256(embeddedExecutor);
if (embeddedExecutorSha !== embeddedSidecar) {
  throw new Error("Embedded executor sidecar mismatch: file=" + embeddedExecutorSha + " sidecar=" + embeddedSidecar);
}

const currentLivingSha = await codeSha(provider, LIVING_ID);
if (currentLivingSha !== PREVIOUS_LIVING_SHA && currentLivingSha !== targetLivingSha) {
  throw new Error("Unexpected current Living code SHA: " + currentLivingSha);
}

const beforeState = await accountState(provider, LIVING_ID);
const beforeCode = await codeBytes(provider, LIVING_ID);
const beforeConfig = await view(living, LIVING_ID, "get_living_game_config");
if (beforeConfig?.nuve_ft_id !== NUVE_ID || beforeConfig?.treasury_id !== TREASURY_ID) {
  throw new Error("Living game config does not match deployment evidence");
}
const beforeSupply = await view(living, LIVING_ID, "nft_total_supply");
const beforeToken1 = await view(living, LIVING_ID, "nft_token", { token_id: "1" });
const beforeLiving1 = await view(living, LIVING_ID, "get_living_state", { token_id: "1" });
const bounds = await view(living, NUVE_ID, "storage_balance_bounds");
const storageMinimum = BigInt(bounds.min);
if (storageMinimum !== EXPECTED_NUVE_STORAGE_MIN) {
  throw new Error("NUVE storage minimum drift: live=" + storageMinimum + " expected=" + EXPECTED_NUVE_STORAGE_MIN);
}

const codeDelta = Math.max(0, livingWasm.length - beforeCode.length);
const projectedStorageUsage = BigInt(beforeState.storage_usage + codeDelta);
const projectedRequiredAmount = projectedStorageUsage * STORAGE_PRICE_PER_BYTE + RELEASE_RESERVE;
const currentAmount = BigInt(beforeState.amount);
const topUp = projectedRequiredAmount > currentAmount ? projectedRequiredAmount - currentAmount : 0n;

console.log("mode=" + (EXECUTE ? "EXECUTE" : "DRY_RUN"));
console.log("living=" + LIVING_ID);
console.log("current_living_sha256=" + currentLivingSha);
console.log("target_living_sha256=" + targetLivingSha);
console.log("target_living_size=" + livingWasm.length);
console.log("embedded_executor_sha256=" + embeddedExecutorSha);
console.log("nuve_storage_min_yocto=" + storageMinimum);
console.log("projected_storage_usage=" + projectedStorageUsage);
console.log("required_top_up_yocto=" + topUp);
console.log("required_top_up_near=" + yoctoToNear(topUp));

if (!EXECUTE) process.exit(0);

const transactions = [];
if (topUp > 0n) {
  if (!FUNDING_ACCOUNT) {
    throw new Error("Living account needs Testnet NEAR top-up; set HACKATHON_FUNDING_ACCOUNT");
  }
  const funding = await near.account(FUNDING_ACCOUNT);
  const fundingState = await accountState(provider, FUNDING_ACCOUNT);
  const fundingReserve = 10n ** 23n;
  if (BigInt(fundingState.amount) < topUp + fundingReserve) {
    throw new Error("Funding account balance is below bounded top-up + reserve");
  }
  const outcome = await funding.sendMoney(LIVING_ID, topUp);
  transactions.push({
    step: "top_up_living",
    from: FUNDING_ACCOUNT,
    to: LIVING_ID,
    amount_yocto: topUp.toString(),
    tx_hash: txHash(outcome),
  });
  await waitForAccountAmount(
    provider,
    LIVING_ID,
    projectedRequiredAmount,
  );
}

if (currentLivingSha !== targetLivingSha) {
  const outcome = await living.deployContract(livingWasm);
  transactions.push({
    step: "deploy_living_smart_account_demo",
    account_id: LIVING_ID,
    wasm_sha256: targetLivingSha,
    tx_hash: txHash(outcome),
  });
}

const readbackSha = await waitForCodeSha(
  provider,
  LIVING_ID,
  targetLivingSha,
);

const afterConfig = await view(living, LIVING_ID, "get_living_game_config");
const afterSupply = await view(living, LIVING_ID, "nft_total_supply");
const afterToken1 = await view(living, LIVING_ID, "nft_token", { token_id: "1" });
const afterLiving1 = await view(living, LIVING_ID, "get_living_state", { token_id: "1" });
const creationDeposit = await view(living, LIVING_ID, "nuvi_account_creation_deposit");
const expectedChild = await view(living, LIVING_ID, "expected_nuvi_account_id", { token_id: "1" });

if (JSON.stringify(afterConfig) !== JSON.stringify(beforeConfig)) {
  throw new Error("Living config changed across code-only upgrade");
}
if (String(afterSupply) !== String(beforeSupply)) {
  throw new Error("NFT supply changed across code-only upgrade");
}
if (afterToken1?.owner_id !== beforeToken1?.owner_id) {
  throw new Error("Token #1 owner changed across code-only upgrade");
}
if (JSON.stringify(afterLiving1) !== JSON.stringify(beforeLiving1)) {
  throw new Error("Token #1 Living state changed across code-only upgrade");
}

const evidence = {
  schema: "nuvi-smart-account-testnet-upgrade-v1",
  generated_at_utc: new Date().toISOString(),
  network_id: NETWORK_ID,
  rpc_url: RPC_URL,
  source_branch: "feature/nuvi-nft-controlled-account-demo",
  living_nft_account_id: LIVING_ID,
  nuve_ft_account_id: NUVE_ID,
  predecessor_living_wasm_sha256: PREVIOUS_LIVING_SHA,
  observed_living_wasm_sha256_before_run: currentLivingSha,
  target_living_wasm_sha256: targetLivingSha,
  upgrade_already_current_before_run: currentLivingSha === targetLivingSha,
  target_living_wasm_size_bytes: livingWasm.length,
  embedded_executor_sha256: embeddedExecutorSha,
  nuve_storage_min_yocto: storageMinimum.toString(),
  nuvi_account_creation_deposit_yocto: String(creationDeposit),
  expected_token_1_account_id: String(expectedChild),
  token_1_owner_before: beforeToken1.owner_id,
  token_1_form_before: beforeLiving1.form_id,
  token_1_mutation_count_before: beforeLiving1.mutation_count,
  post_upgrade_state_preserved: true,
  transactions,
};
await writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n", "utf8");
console.log("verified_living_sha256=" + readbackSha);
console.log("nuvi_account_creation_deposit_yocto=" + creationDeposit);
console.log("expected_token_1_account_id=" + expectedChild);
console.log("evidence=deployments/smart-account-upgrade-testnet.json");
