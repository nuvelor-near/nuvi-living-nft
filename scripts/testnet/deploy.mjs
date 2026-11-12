import { connect, keyStores, KeyPair } from "near-api-js";
import {
  access,
  chmod,
  mkdir,
  readFile,
  rename,
  writeFile,
} from "node:fs/promises";
import { constants as fsConstants } from "node:fs";
import { createHash } from "node:crypto";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NETWORK_ID = process.env.NEAR_NETWORK_ID || "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const PARENT_ID =
  process.env.HACKATHON_PARENT_ACCOUNT || "nvlborc70517034832.testnet";
const EXECUTE = process.env.EXECUTE === "1";

const LIVING_ID =
  process.env.HACKATHON_LIVING_ACCOUNT || `nuvi-hack26.${PARENT_ID}`;
const NUVE_ID =
  process.env.HACKATHON_NUVE_ACCOUNT || `nuve-hack26.${PARENT_ID}`;
const TREASURY_ID =
  process.env.HACKATHON_TREASURY_ACCOUNT || `treasury-hack26.${PARENT_ID}`;
const PLAYER_ID =
  process.env.HACKATHON_PLAYER_ACCOUNT || `player-hack26.${PARENT_ID}`;

const ALLOCATIONS = {
  [LIVING_ID]: process.env.HACKATHON_LIVING_BALANCE || "7.3",
  [NUVE_ID]: process.env.HACKATHON_NUVE_BALANCE || "2.6",
  [TREASURY_ID]: process.env.HACKATHON_TREASURY_BALANCE || "0.3",
  [PLAYER_ID]: process.env.HACKATHON_PLAYER_BALANCE || "0.8",
};

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const credentialDir =
  process.env.NEAR_CREDENTIAL_DIR ||
  path.join(homedir(), ".near-credentials", NETWORK_ID);
const parentCredentialPath =
  process.env.HACKATHON_PARENT_CREDENTIAL ||
  path.join(credentialDir, `${PARENT_ID}.json`);

const livingWasmPath = path.join(
  repoRoot,
  "contracts/living-nft/target/near/nuvelor_nft.wasm",
);
const nuveWasmPath = path.join(
  repoRoot,
  "contracts/nuve-ft/target/near/nuvi_demo_nuve_ft.wasm",
);
const evidencePath = path.join(repoRoot, "deployments/testnet.json");

const TGAS = 1_000_000_000_000n;
const ONE_NEAR = 10n ** 24n;
const STORAGE_PRICE_PER_BYTE = 10n ** 19n;

function yoctoNear(value) {
  const raw = String(value).trim();
  if (!/^\d+(\.\d+)?$/.test(raw)) {
    throw new Error(`Invalid NEAR amount: ${value}`);
  }
  const [whole, fraction = ""] = raw.split(".");
  const padded = (fraction + "0".repeat(24)).slice(0, 24);
  return BigInt(whole) * ONE_NEAR + BigInt(padded);
}

function formatNear(value) {
  const amount = BigInt(value);
  const whole = amount / ONE_NEAR;
  const fraction = (amount % ONE_NEAR).toString().padStart(24, "0").slice(0, 4);
  return `${whole}.${fraction}`;
}

function txHash(outcome) {
  return outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
}

function sha256(data) {
  return createHash("sha256").update(data).digest("hex");
}

async function fileExists(file) {
  try {
    await access(file, fsConstants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function loadCredential(file, expectedAccountId) {
  const value = JSON.parse(await readFile(file, "utf8"));
  if (value.account_id !== expectedAccountId) {
    throw new Error(`Credential account mismatch for ${expectedAccountId}`);
  }
  if (!value.public_key || !value.private_key) {
    throw new Error(`Incomplete credential for ${expectedAccountId}`);
  }
  return value;
}

async function writePendingCredential(accountId, keyPair) {
  await mkdir(credentialDir, { recursive: true });
  const finalPath = path.join(credentialDir, `${accountId}.json`);
  const pendingPath = `${finalPath}.pending-hackathon`;

  if (await fileExists(finalPath)) {
    return { credential: await loadCredential(finalPath, accountId), finalPath, pendingPath };
  }
  if (await fileExists(pendingPath)) {
    return { credential: await loadCredential(pendingPath, accountId), finalPath, pendingPath };
  }

  const credential = {
    account_id: accountId,
    public_key: keyPair.getPublicKey().toString(),
    private_key: keyPair.toString(),
  };

  await writeFile(pendingPath, JSON.stringify(credential, null, 2) + "\n", {
    flag: "wx",
    mode: 0o600,
  });
  await chmod(pendingPath, 0o600);
  return { credential, finalPath, pendingPath };
}

async function accountState(provider, accountId) {
  try {
    return await provider.query({
      request_type: "view_account",
      finality: "final",
      account_id: accountId,
    });
  } catch (error) {
    const message = String(error?.message || error);
    if (
      error?.type === "AccountDoesNotExist" ||
      message.includes("UNKNOWN_ACCOUNT") ||
      message.includes("does not exist") ||
      message.includes("doesn't exist") ||
      message.includes("Unknown Account")
    ) {
      return null;
    }
    throw error;
  }
}

async function codeSha(provider, accountId) {
  const state = await accountState(provider, accountId);
  if (!state) return null;
  if (state.code_hash === "11111111111111111111111111111111") return null;

  const code = await provider.query({
    request_type: "view_code",
    finality: "final",
    account_id: accountId,
  });
  return sha256(Buffer.from(code.code_base64, "base64"));
}

async function view(account, contractId, methodName, args = {}) {
  return account.viewFunction({
    contractId,
    methodName,
    args,
    blockQuery: { finality: "final" },
  });
}

async function main() {
  const parentCredential = await loadCredential(parentCredentialPath, PARENT_ID);
  const keyStore = new keyStores.InMemoryKeyStore();
  await keyStore.setKey(
    NETWORK_ID,
    PARENT_ID,
    KeyPair.fromString(parentCredential.private_key),
  );

  const near = await connect({
    networkId: NETWORK_ID,
    nodeUrl: RPC_URL,
    keyStore,
  });
  const provider = near.connection.provider;
  const parent = await near.account(PARENT_ID);

  const livingWasm = await readFile(livingWasmPath);
  const nuveWasm = await readFile(nuveWasmPath);
  const livingWasmSha = sha256(livingWasm);
  const nuveWasmSha = sha256(nuveWasm);

  const minimumLiving = BigInt(livingWasm.length) * STORAGE_PRICE_PER_BYTE + yoctoNear("0.25");
  const minimumNuve = BigInt(nuveWasm.length) * STORAGE_PRICE_PER_BYTE + yoctoNear("0.20");
  if (yoctoNear(ALLOCATIONS[LIVING_ID]) < minimumLiving) {
    throw new Error("Living account allocation is below code-storage safety floor");
  }
  if (yoctoNear(ALLOCATIONS[NUVE_ID]) < minimumNuve) {
    throw new Error("NUVE account allocation is below code-storage safety floor");
  }

  const targetIds = [LIVING_ID, NUVE_ID, TREASURY_ID, PLAYER_ID];
  const states = {};
  for (const accountId of targetIds) {
    states[accountId] = await accountState(provider, accountId);
  }

  const parentState = await accountState(provider, PARENT_ID);
  let required = 0n;
  for (const accountId of targetIds) {
    if (!states[accountId]) required += yoctoNear(ALLOCATIONS[accountId]);
  }
  const reserve = yoctoNear("0.50");
  if (BigInt(parentState.amount) < required + reserve) {
    throw new Error(
      `Parent balance ${formatNear(parentState.amount)} NEAR is below required ${formatNear(required + reserve)} NEAR`,
    );
  }

  console.log("mode=" + (EXECUTE ? "EXECUTE" : "DRY_RUN"));
  console.log("parent=" + PARENT_ID);
  console.log("parent_balance_near=" + formatNear(parentState.amount));
  console.log("living=" + LIVING_ID);
  console.log("nuve=" + NUVE_ID);
  console.log("treasury=" + TREASURY_ID);
  console.log("player=" + PLAYER_ID);
  console.log("living_wasm_sha256=" + livingWasmSha);
  console.log("nuve_wasm_sha256=" + nuveWasmSha);
  console.log("missing_account_allocation_near=" + formatNear(required));

  if (!EXECUTE) {
    for (const accountId of targetIds) {
      console.log(
        `account:${accountId}=${states[accountId] ? "EXISTS" : `CREATE:${ALLOCATIONS[accountId]}`}`,
      );
    }
    return;
  }

  const transactions = [];

  async function ensureAccount(accountId) {
    const finalPath = path.join(credentialDir, `${accountId}.json`);
    const pendingPath = `${finalPath}.pending-hackathon`;
    const existingState = await accountState(provider, accountId);

    if (existingState) {
      let credentialPath = null;
      if (await fileExists(finalPath)) credentialPath = finalPath;
      else if (await fileExists(pendingPath)) {
        await rename(pendingPath, finalPath);
        credentialPath = finalPath;
      }
      if (!credentialPath) {
        throw new Error(
          `Account ${accountId} exists but its local credential is unavailable; refusing to overwrite`,
        );
      }
      const credential = await loadCredential(credentialPath, accountId);
      await keyStore.setKey(
        NETWORK_ID,
        accountId,
        KeyPair.fromString(credential.private_key),
      );
      console.log("account_exists=" + accountId);
      return;
    }

    let prepared;
    if (await fileExists(finalPath)) {
      const credential = await loadCredential(finalPath, accountId);
      prepared = { credential, finalPath, pendingPath };
    } else if (await fileExists(pendingPath)) {
      const credential = await loadCredential(pendingPath, accountId);
      prepared = { credential, finalPath, pendingPath };
    } else {
      prepared = await writePendingCredential(
        accountId,
        KeyPair.fromRandom("ed25519"),
      );
    }

    const publicKey = KeyPair.fromString(
      prepared.credential.private_key,
    ).getPublicKey();
    const outcome = await parent.createAccount(
      accountId,
      publicKey,
      yoctoNear(ALLOCATIONS[accountId]),
    );
    transactions.push({
      step: "create_account",
      account_id: accountId,
      tx_hash: txHash(outcome),
    });

    if (await fileExists(prepared.pendingPath)) {
      await rename(prepared.pendingPath, prepared.finalPath);
    }
    await chmod(prepared.finalPath, 0o600);
    await keyStore.setKey(
      NETWORK_ID,
      accountId,
      KeyPair.fromString(prepared.credential.private_key),
    );
    console.log("account_created=" + accountId);
  }

  for (const accountId of targetIds) {
    await ensureAccount(accountId);
  }

  async function ensureCode(accountId, wasm, expectedSha) {
    const currentSha = await codeSha(provider, accountId);
    if (currentSha === expectedSha) {
      console.log("code_current=" + accountId);
      return;
    }
    if (currentSha && currentSha !== expectedSha) {
      throw new Error(
        `Account ${accountId} already has different code; refusing redeploy`,
      );
    }

    const account = await near.account(accountId);
    const outcome = await account.deployContract(wasm);
    transactions.push({
      step: "deploy_contract",
      account_id: accountId,
      tx_hash: txHash(outcome),
      wasm_sha256: expectedSha,
    });

    const readback = await codeSha(provider, accountId);
    if (readback !== expectedSha) {
      throw new Error(`WASM readback mismatch for ${accountId}`);
    }
    console.log("code_deployed=" + accountId);
  }

  await ensureCode(NUVE_ID, nuveWasm, nuveWasmSha);
  await ensureCode(LIVING_ID, livingWasm, livingWasmSha);

  const nuve = await near.account(NUVE_ID);
  const living = await near.account(LIVING_ID);

  let nuveInitialized = false;
  try {
    const owner = await view(nuve, NUVE_ID, "owner_id");
    const minter = await view(nuve, NUVE_ID, "reward_minter_id");
    if (owner !== PARENT_ID || minter !== LIVING_ID) {
      throw new Error("Existing NUVE state does not match Hackathon configuration");
    }
    nuveInitialized = true;
  } catch (error) {
    if (String(error?.message || error).includes("does not match")) throw error;
  }

  if (!nuveInitialized) {
    const outcome = await nuve.functionCall({
      contractId: NUVE_ID,
      methodName: "new",
      args: {
        owner_id: PARENT_ID,
        reward_minter_id: LIVING_ID,
      },
      gas: 100n * TGAS,
      attachedDeposit: 0n,
    });
    transactions.push({
      step: "init_nuve",
      account_id: NUVE_ID,
      tx_hash: txHash(outcome),
    });
    console.log("initialized=" + NUVE_ID);
  } else {
    console.log("initialized_current=" + NUVE_ID);
  }

  let livingInitialized = false;
  try {
    const config = await view(living, LIVING_ID, "get_living_game_config");
    if (
      config?.nuve_ft_id !== NUVE_ID ||
      config?.treasury_id !== TREASURY_ID
    ) {
      throw new Error("Existing Living state does not match Hackathon configuration");
    }
    livingInitialized = true;
  } catch (error) {
    if (String(error?.message || error).includes("does not match")) throw error;
  }

  if (!livingInitialized) {
    const outcome = await living.functionCall({
      contractId: LIVING_ID,
      methodName: "new",
      args: {
        config: {
          owner_id: PARENT_ID,
          metadata: {
            spec: "nft-1.0.0",
            name: "NUVI Living NFT",
            symbol: "NUVI",
            icon: null,
            base_uri: null,
            reference: null,
            reference_hash: null,
          },
          max_batch: 100,
          max_tokens_per_view: 200,
          max_approvals_per_token: 10,
          max_royalty_bps: 2000,
          max_payout_len: 10,
          max_metadata_extra_bytes: 2048,
          min_storage_deposit: null,
        },
        nuve_ft_id: NUVE_ID,
        treasury_id: TREASURY_ID,
      },
      gas: 120n * TGAS,
      attachedDeposit: 0n,
    });
    transactions.push({
      step: "init_living",
      account_id: LIVING_ID,
      tx_hash: txHash(outcome),
    });
    console.log("initialized=" + LIVING_ID);
  } else {
    console.log("initialized_current=" + LIVING_ID);
  }

  const storageBounds = await view(
    parent,
    NUVE_ID,
    "storage_balance_bounds",
  );
  const storageMinimum = BigInt(storageBounds.min);

  for (const accountId of [LIVING_ID, TREASURY_ID]) {
    const existing = await view(parent, NUVE_ID, "storage_balance_of", {
      account_id: accountId,
    });
    if (existing) {
      console.log("ft_storage_current=" + accountId);
      continue;
    }

    const outcome = await parent.functionCall({
      contractId: NUVE_ID,
      methodName: "storage_deposit",
      args: {
        account_id: accountId,
        registration_only: true,
      },
      gas: 40n * TGAS,
      attachedDeposit: storageMinimum,
    });
    transactions.push({
      step: "register_ft_storage",
      account_id: accountId,
      tx_hash: txHash(outcome),
    });
    console.log("ft_storage_registered=" + accountId);
  }

  const verifiedNuveOwner = await view(nuve, NUVE_ID, "owner_id");
  const verifiedRewardMinter = await view(nuve, NUVE_ID, "reward_minter_id");
  const verifiedGameConfig = await view(
    living,
    LIVING_ID,
    "get_living_game_config",
  );

  const accountBalances = {};
  for (const accountId of [PARENT_ID, ...targetIds]) {
    const state = await accountState(provider, accountId);
    accountBalances[accountId] = state ? state.amount : null;
  }

  const evidence = {
    schema: "nuvi-hackathon-testnet-deployment-v1",
    generated_at_utc: new Date().toISOString(),
    network_id: NETWORK_ID,
    rpc_url: RPC_URL,
    parent_account_id: PARENT_ID,
    living_nft_account_id: LIVING_ID,
    nuve_ft_account_id: NUVE_ID,
    treasury_account_id: TREASURY_ID,
    e2e_player_account_id: PLAYER_ID,
    living_nft_wasm: {
      sha256: livingWasmSha,
      size_bytes: livingWasm.length,
    },
    nuve_ft_wasm: {
      sha256: nuveWasmSha,
      size_bytes: nuveWasm.length,
    },
    verified: {
      nuve_owner_id: verifiedNuveOwner,
      reward_minter_id: verifiedRewardMinter,
      living_game_config: verifiedGameConfig,
      living_code_sha256: await codeSha(provider, LIVING_ID),
      nuve_code_sha256: await codeSha(provider, NUVE_ID),
      account_balances_yocto: accountBalances,
    },
    transactions,
  };

  await mkdir(path.dirname(evidencePath), { recursive: true });
  await writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n", "utf8");
  console.log("evidence=" + path.relative(repoRoot, evidencePath));
}

await main();
