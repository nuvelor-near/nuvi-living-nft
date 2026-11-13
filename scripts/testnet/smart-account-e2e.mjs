import { connect, keyStores, KeyPair } from "near-api-js";
import { readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NETWORK_ID = process.env.NEAR_NETWORK_ID || "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const EXECUTE = process.env.EXECUTE === "1";
const FUNDING_ACCOUNT = process.env.HACKATHON_FUNDING_ACCOUNT || "";
const TGAS = 1_000_000_000_000n;
const ONE_NEAR = 10n ** 24n;
const NUVE_UNIT = 10_000n;
const PLAYER_RESERVE = 4n * 10n ** 23n;

if (NETWORK_ID !== "testnet") throw new Error("This E2E is Testnet-only");

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const deployment = JSON.parse(
  await readFile(path.join(repoRoot, "deployments/testnet.json"), "utf8"),
);
const evidencePath = path.join(repoRoot, "deployments/smart-account-e2e-testnet.json");

const LIVING_ID = deployment.living_nft_account_id;
const NUVE_ID = deployment.nuve_ft_account_id;
const PLAYER_ID = deployment.e2e_player_account_id;
const SECOND_OWNER_ID = deployment.parent_account_id;
const TOKEN_ID = process.env.HACKATHON_NUVI_TOKEN_ID || "1";

function txHash(value) {
  return (
    value?.transaction?.hash ||
    value?.transaction_outcome?.id ||
    value?.final_execution_outcome?.transaction?.hash ||
    value?.context?.transaction_hash ||
    null
  );
}
function parseExtra(token) {
  try {
    return JSON.parse(token?.metadata?.extra || "{}");
  } catch {
    return {};
  }
}
async function loadCredential(accountId) {
  const file = path.join(homedir(), ".near-credentials", NETWORK_ID, accountId + ".json");
  const value = JSON.parse(await readFile(file, "utf8"));
  if (value.account_id !== accountId || !value.private_key || !value.public_key) {
    throw new Error("Credential missing or mismatched for " + accountId);
  }
  return value;
}
async function addCredential(keyStore, accountId) {
  const credential = await loadCredential(accountId);
  await keyStore.setKey(NETWORK_ID, accountId, KeyPair.fromString(credential.private_key));
}
async function accountState(provider, accountId) {
  try {
    return await provider.query({
      request_type: "view_account",
      finality: "final",
      account_id: accountId,
    });
  } catch {
    return null;
  }
}
async function accessKeys(provider, accountId) {
  const result = await provider.query({
    request_type: "view_access_key_list",
    finality: "final",
    account_id: accountId,
  });
  return Array.isArray(result?.keys) ? result.keys : [];
}
async function view(account, contractId, methodName, args = {}) {
  return account.viewFunction({
    contractId,
    methodName,
    args,
    blockQuery: { finality: "final" },
  });
}
async function call(
  account,
  { contractId, methodName, args = {}, gasTgas = 100, attachedDeposit = 0n },
) {
  return account.functionCall({
    contractId,
    methodName,
    args,
    gas: BigInt(gasTgas) * TGAS,
    attachedDeposit,
  });
}
async function expectedFailure(label, operation) {
  try {
    const outcome = await operation();
    throw new Error(label + " unexpectedly succeeded: " + (txHash(outcome) || "unknown tx"));
  } catch (error) {
    const message = String(error?.message || error);
    if (message.includes("unexpectedly succeeded")) throw error;
    return {
      label,
      rejected: true,
      tx_hash: txHash(error),
      error: message.slice(0, 800),
    };
  }
}
async function poll(read, predicate, label, timeoutMs = 45_000) {
  const started = Date.now();
  let last;
  while (Date.now() - started < timeoutMs) {
    last = await read();
    if (predicate(last)) return last;
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error("Timed out waiting for " + label + ": " + JSON.stringify(last));
}

const keyStore = new keyStores.InMemoryKeyStore();
await addCredential(keyStore, PLAYER_ID);
await addCredential(keyStore, SECOND_OWNER_ID);
if (FUNDING_ACCOUNT) await addCredential(keyStore, FUNDING_ACCOUNT);

const near = await connect({ networkId: NETWORK_ID, nodeUrl: RPC_URL, keyStore });
const provider = near.connection.provider;
const player = await near.account(PLAYER_ID);
const secondOwner = await near.account(SECOND_OWNER_ID);
const reader = player;

const tokenBefore = await view(reader, LIVING_ID, "nft_token", { token_id: TOKEN_ID });
if (!tokenBefore) throw new Error("NUVI token does not exist");
if (tokenBefore.owner_id !== PLAYER_ID) {
  throw new Error("Expected token owner " + PLAYER_ID + ", got " + tokenBefore.owner_id);
}
const livingBefore = await view(reader, LIVING_ID, "get_living_state", { token_id: TOKEN_ID });
if (Number(livingBefore?.mutation_count || 0) < 1) {
  throw new Error("Compatibility E2E expects an already-mutated NUVI");
}
const expectedChild = await view(reader, LIVING_ID, "expected_nuvi_account_id", { token_id: TOKEN_ID });
const provisioningDeposit = BigInt(await view(reader, LIVING_ID, "nuvi_account_creation_deposit"));
const playerStorage = await view(reader, NUVE_ID, "storage_balance_of", { account_id: PLAYER_ID });
if (!playerStorage) throw new Error("E2E player is not NUVE storage-registered");

const extraBefore = parseExtra(tokenBefore);
const statusBefore = String(extraBefore.nuvi_account_status || "");
let topUp = 0n;
if (!statusBefore) {
  const playerState = await accountState(provider, PLAYER_ID);
  const requiredPlayerAmount = provisioningDeposit + PLAYER_RESERVE;
  const currentPlayerAmount = BigInt(playerState.amount);
  topUp = requiredPlayerAmount > currentPlayerAmount
    ? requiredPlayerAmount - currentPlayerAmount
    : 0n;
}

console.log("mode=" + (EXECUTE ? "EXECUTE" : "DRY_RUN"));
console.log("token_id=" + TOKEN_ID);
console.log("token_owner=" + tokenBefore.owner_id);
console.log("expected_child=" + expectedChild);
console.log("status_before=" + (statusBefore || "none"));
console.log("provisioning_deposit_yocto=" + provisioningDeposit);
console.log("player_top_up_yocto=" + topUp);

if (!EXECUTE) process.exit(0);

const transactions = [];
const rejected = [];

async function persistProgress(status, details = {}) {
  const snapshot = {
    schema: "nuvi-smart-account-testnet-e2e-progress-v1",
    updated_at_utc: new Date().toISOString(),
    status,
    network_id: NETWORK_ID,
    token_id: TOKEN_ID,
    original_owner: PLAYER_ID,
    second_owner: SECOND_OWNER_ID,
    expected_child_account_id: String(expectedChild),
    provisioning_deposit_yocto: provisioningDeposit.toString(),
    transactions,
    rejected,
    ...details,
  };
  await writeFile(evidencePath, JSON.stringify(snapshot, null, 2) + "\n", "utf8");
}

if (topUp > 0n) {
  if (!FUNDING_ACCOUNT) {
    throw new Error("Player needs Testnet NEAR top-up; set HACKATHON_FUNDING_ACCOUNT");
  }
  const funding = await near.account(FUNDING_ACCOUNT);
  const fundingState = await accountState(provider, FUNDING_ACCOUNT);
  if (BigInt(fundingState.amount) < topUp + ONE_NEAR / 10n) {
    throw new Error("Funding account balance is below player top-up + reserve");
  }
  const outcome = await funding.sendMoney(PLAYER_ID, topUp);
  transactions.push({
    step: "top_up_player",
    from: FUNDING_ACCOUNT,
    to: PLAYER_ID,
    amount_yocto: topUp.toString(),
    tx_hash: txHash(outcome),
  });
  await persistProgress("PLAYER_FUNDED");
}

if (!statusBefore) {
  const outcome = await call(player, {
    contractId: LIVING_ID,
    methodName: "provision_nuvi_account",
    args: { token_id: TOKEN_ID },
    gasTgas: 220,
    attachedDeposit: provisioningDeposit,
  });
  transactions.push({
    step: "provision_nuvi_account",
    caller: PLAYER_ID,
    token_id: TOKEN_ID,
    tx_hash: txHash(outcome),
  });
  await persistProgress("PROVISION_SUBMITTED");
} else if (statusBefore === "failed") {
  throw new Error("Existing NUVI Account provisioning status is failed; refusing replay");
}

const activeToken = await poll(
  () => view(reader, LIVING_ID, "nft_token", { token_id: TOKEN_ID }),
  (token) => {
    const extra = parseExtra(token);
    return extra.nuvi_account_status === "active" && extra.nuvi_account_id === expectedChild;
  },
  "NUVI Account active metadata",
);
const activeExtra = parseExtra(activeToken);
await persistProgress("ACCOUNT_ACTIVE", {
  metadata_status: activeExtra.nuvi_account_status,
  metadata_nuvi_account_id: activeExtra.nuvi_account_id,
});

const childState = await poll(
  () => accountState(provider, expectedChild),
  Boolean,
  "child account existence",
);
if (!childState.code_hash || childState.code_hash === "11111111111111111111111111111111") {
  throw new Error("Child account has no deployed code");
}
const childKeys = await accessKeys(provider, expectedChild);
if (childKeys.length !== 0) throw new Error("Child account unexpectedly has access keys");

const executorConfig = await view(reader, expectedChild, "get_config");
if (
  executorConfig?.nft_contract_id !== LIVING_ID ||
  String(executorConfig?.token_id || "") !== TOKEN_ID ||
  executorConfig?.nuve_ft_id !== NUVE_ID
) {
  throw new Error("Child executor binding mismatch");
}
const childStorage = await view(reader, NUVE_ID, "storage_balance_of", { account_id: expectedChild });
if (!childStorage) throw new Error("Child is not NUVE storage-registered");

const initialChildBalance = BigInt(
  await view(reader, NUVE_ID, "ft_balance_of", { account_id: expectedChild }),
);
if (initialChildBalance < 2n * NUVE_UNIT) {
  throw new Error("Child NUVE balance is too small for two proof transfers");
}

const ownerSend = await call(player, {
  contractId: expectedChild,
  methodName: "request_ft_transfer",
  args: { receiver_id: PLAYER_ID, amount: NUVE_UNIT.toString() },
  gasTgas: 120,
});
transactions.push({
  step: "owner_send_before_nft_transfer",
  caller: PLAYER_ID,
  from: expectedChild,
  to: PLAYER_ID,
  amount_atomic: NUVE_UNIT.toString(),
  tx_hash: txHash(ownerSend),
});
await persistProgress("OWNER_SEND_PASS");
const afterOwnerSendBalance = BigInt(
  await view(reader, NUVE_ID, "ft_balance_of", { account_id: expectedChild }),
);
if (afterOwnerSendBalance !== initialChildBalance - NUVE_UNIT) {
  throw new Error("Authorized owner transfer did not reduce child balance by 1 NUVE");
}

rejected.push(
  await expectedFailure("non_owner_before_nft_transfer", () =>
    call(secondOwner, {
      contractId: expectedChild,
      methodName: "request_ft_transfer",
      args: { receiver_id: PLAYER_ID, amount: NUVE_UNIT.toString() },
      gasTgas: 120,
    }),
  ),
);

let transferredToSecondOwner = false;
try {
  const transfer = await call(player, {
    contractId: LIVING_ID,
    methodName: "nft_transfer",
    args: {
      receiver_id: SECOND_OWNER_ID,
      token_id: TOKEN_ID,
      approval_id: null,
      memo: "NUVI smart-account ownership-control proof",
    },
    gasTgas: 60,
    attachedDeposit: 1n,
  });
  transferredToSecondOwner = true;
  transactions.push({
    step: "transfer_nuvi_to_second_owner",
    from: PLAYER_ID,
    to: SECOND_OWNER_ID,
    token_id: TOKEN_ID,
    tx_hash: txHash(transfer),
  });
  await persistProgress("NFT_TRANSFERRED_TO_SECOND_OWNER");

  const transferredToken = await poll(
    () => view(reader, LIVING_ID, "nft_token", { token_id: TOKEN_ID }),
    (token) => token?.owner_id === SECOND_OWNER_ID,
    "NFT ownership transfer",
  );
  const transferredExtra = parseExtra(transferredToken);
  if (transferredExtra.nuvi_account_id !== expectedChild) {
    throw new Error("NUVI Account metadata binding changed across NFT transfer");
  }
  const balanceAfterOwnershipTransfer = BigInt(
    await view(reader, NUVE_ID, "ft_balance_of", { account_id: expectedChild }),
  );
  if (balanceAfterOwnershipTransfer !== afterOwnerSendBalance) {
    throw new Error("Child balance changed merely because NFT ownership changed");
  }

  rejected.push(
    await expectedFailure("old_owner_after_nft_transfer", () =>
      call(player, {
        contractId: expectedChild,
        methodName: "request_ft_transfer",
        args: { receiver_id: PLAYER_ID, amount: NUVE_UNIT.toString() },
        gasTgas: 120,
      }),
    ),
  );

  const newOwnerSend = await call(secondOwner, {
    contractId: expectedChild,
    methodName: "request_ft_transfer",
    args: { receiver_id: PLAYER_ID, amount: NUVE_UNIT.toString() },
    gasTgas: 120,
  });
  transactions.push({
    step: "new_owner_send_after_nft_transfer",
    caller: SECOND_OWNER_ID,
    from: expectedChild,
    to: PLAYER_ID,
    amount_atomic: NUVE_UNIT.toString(),
    tx_hash: txHash(newOwnerSend),
  });
  await persistProgress("NEW_OWNER_SEND_PASS");

  const afterNewOwnerSendBalance = BigInt(
    await view(reader, NUVE_ID, "ft_balance_of", { account_id: expectedChild }),
  );
  if (afterNewOwnerSendBalance !== balanceAfterOwnershipTransfer - NUVE_UNIT) {
    throw new Error("New owner transfer did not reduce child balance by 1 NUVE");
  }
} finally {
  if (transferredToSecondOwner) {
    const current = await view(reader, LIVING_ID, "nft_token", { token_id: TOKEN_ID });
    if (current?.owner_id === SECOND_OWNER_ID) {
      const restore = await call(secondOwner, {
        contractId: LIVING_ID,
        methodName: "nft_transfer",
        args: {
          receiver_id: PLAYER_ID,
          token_id: TOKEN_ID,
          approval_id: null,
          memo: "Restore Hackathon demo owner after control proof",
        },
        gasTgas: 60,
        attachedDeposit: 1n,
      });
      transactions.push({
        step: "restore_nuvi_owner",
        from: SECOND_OWNER_ID,
        to: PLAYER_ID,
        token_id: TOKEN_ID,
        tx_hash: txHash(restore),
      });
      await persistProgress("OWNER_RESTORED");
    }
  }
}

const finalToken = await poll(
  () => view(reader, LIVING_ID, "nft_token", { token_id: TOKEN_ID }),
  (token) => token?.owner_id === PLAYER_ID,
  "restored NFT owner",
);
const finalExtra = parseExtra(finalToken);
const finalChildBalance = await view(reader, NUVE_ID, "ft_balance_of", {
  account_id: expectedChild,
});
const finalChildState = await accountState(provider, expectedChild);
const finalKeys = await accessKeys(provider, expectedChild);

const evidence = {
  schema: "nuvi-smart-account-testnet-e2e-v1",
  generated_at_utc: new Date().toISOString(),
  network_id: NETWORK_ID,
  rpc_url: RPC_URL,
  token_id: TOKEN_ID,
  original_and_restored_owner: PLAYER_ID,
  second_owner: SECOND_OWNER_ID,
  child_account_id: expectedChild,
  child_code_hash: finalChildState.code_hash,
  child_access_key_count: finalKeys.length,
  executor_config: executorConfig,
  nft_metadata_nuvi_account_id: finalExtra.nuvi_account_id,
  nft_metadata_status: finalExtra.nuvi_account_status,
  nuve_storage_registered: Boolean(childStorage),
  initial_child_nuve_balance_atomic: initialChildBalance.toString(),
  final_child_nuve_balance_atomic: String(finalChildBalance),
  ownership_transfer_preserved_child_address: true,
  ownership_transfer_preserved_balance_before_new_owner_action: true,
  old_owner_rejected_after_transfer: rejected.some(
    (item) => item.label === "old_owner_after_nft_transfer" && item.rejected,
  ),
  new_owner_authorized_after_transfer: transactions.some(
    (item) => item.step === "new_owner_send_after_nft_transfer",
  ),
  final_owner_restored: finalToken.owner_id === PLAYER_ID,
  transactions,
  rejected,
};

await writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n", "utf8");
console.log("child_account=" + expectedChild);
console.log("child_access_keys=" + finalKeys.length);
console.log("final_child_nuve_balance_atomic=" + finalChildBalance);
console.log("final_owner=" + finalToken.owner_id);
console.log("evidence=deployments/smart-account-e2e-testnet.json");
