import { connect, keyStores, KeyPair } from "near-api-js";
import { readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const NETWORK_ID = "testnet";
const RPC_URL = process.env.NEAR_RPC_URL || "https://rpc.testnet.fastnear.com";
const EXECUTE = process.env.EXECUTE === "1";
const PUBLIC_FORMS_BASE_URI = (
  process.env.PUBLIC_FORMS_BASE_URI ||
  "https://nuvelor-near.github.io/nuvi-living-nft/forms"
).replace(/\/$/, "");

if (!PUBLIC_FORMS_BASE_URI.startsWith("https://")) {
  throw new Error("PUBLIC_FORMS_BASE_URI must use HTTPS");
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const evidencePath = path.join(repoRoot, "deployments/testnet.json");
const evidence = JSON.parse(await readFile(evidencePath, "utf8"));

const OWNER_ID = evidence.parent_account_id;
const LIVING_ID = evidence.living_nft_account_id;
const credentialPath = path.join(
  homedir(),
  ".near-credentials",
  NETWORK_ID,
  `${OWNER_ID}.json`,
);

const credential = JSON.parse(await readFile(credentialPath, "utf8"));
if (
  credential.account_id !== OWNER_ID ||
  !credential.private_key ||
  !credential.public_key
) {
  throw new Error("Owner credential is missing or mismatched");
}

const keyStore = new keyStores.InMemoryKeyStore();
await keyStore.setKey(
  NETWORK_ID,
  OWNER_ID,
  KeyPair.fromString(credential.private_key),
);

const near = await connect({
  networkId: NETWORK_ID,
  nodeUrl: RPC_URL,
  keyStore,
});
const owner = await near.account(OWNER_ID);
const TGAS = 1_000_000_000_000n;

async function view(methodName, args = {}) {
  return owner.viewFunction({
    contractId: LIVING_ID,
    methodName,
    args,
    blockQuery: { finality: "final" },
  });
}

const before = await view("nft_metadata");
console.log("mode=" + (EXECUTE ? "EXECUTE" : "DRY_RUN"));
console.log("living_nft=" + LIVING_ID);
console.log("current_base_uri=" + (before.base_uri || ""));
console.log("requested_base_uri=" + PUBLIC_FORMS_BASE_URI);

let txHash = null;

if (before.base_uri !== PUBLIC_FORMS_BASE_URI) {
  if (!EXECUTE) {
    console.log("change_required=true");
    process.exit(0);
  }

  const outcome = await owner.functionCall({
    contractId: LIVING_ID,
    methodName: "update_contract_metadata",
    args: {
      metadata: {
        ...before,
        base_uri: PUBLIC_FORMS_BASE_URI,
      },
    },
    gas: 50n * TGAS,
    attachedDeposit: 1n,
  });
  txHash =
    outcome?.transaction?.hash || outcome?.transaction_outcome?.id || null;
  console.log("tx_hash=" + txHash);
} else {
  console.log("change_required=false");
}

const after = await view("nft_metadata");
if (after.base_uri !== PUBLIC_FORMS_BASE_URI) {
  throw new Error("Collection base_uri readback mismatch");
}

const token = await view("nft_token", { token_id: "1" });
const state = await view("get_living_state", { token_id: "1" });
const expectedMedia = `${PUBLIC_FORMS_BASE_URI}/${state.form_id}.png`;

if (token?.metadata?.media !== expectedMedia) {
  throw new Error(
    `Dynamic media mismatch: expected ${expectedMedia}, got ${token?.metadata?.media}`,
  );
}

evidence.media = {
  updated_at_utc: new Date().toISOString(),
  base_uri: PUBLIC_FORMS_BASE_URI,
  token_id: "1",
  form_id: state.form_id,
  verified_media: token.metadata.media,
  tx_hash: txHash,
};
await writeFile(evidencePath, JSON.stringify(evidence, null, 2) + "\n", "utf8");

console.log("form_id=" + state.form_id);
console.log("verified_media=" + token.metadata.media);
console.log("evidence=deployments/testnet.json");
