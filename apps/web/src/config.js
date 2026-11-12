const env = import.meta.env;

function value(name, fallback = "") {
  const raw = env[name];
  return raw == null || String(raw).trim() === "" ? fallback : String(raw).trim();
}

export const appConfig = Object.freeze({
  networkId: value("VITE_NEAR_NETWORK_ID", "testnet"),
  rpcUrl: value("VITE_NEAR_RPC_URL", "https://rpc.testnet.near.org"),
  nftContractId: value("VITE_NFT_CONTRACT_ID"),
  nuveContractId: value("VITE_NUVE_CONTRACT_ID"),
  treasuryId: value("VITE_TREASURY_ID"),
  explorerAccountBase: value(
    "VITE_EXPLORER_ACCOUNT_BASE",
    "https://testnet.nearblocks.io/address",
  ),
  explorerTxBase: value(
    "VITE_EXPLORER_TX_BASE",
    "https://testnet.nearblocks.io/txns",
  ),
});

export const contractsConfigured = Boolean(
  appConfig.nftContractId && appConfig.nuveContractId && appConfig.treasuryId,
);

export function accountExplorerUrl(accountId) {
  return accountId
    ? `${appConfig.explorerAccountBase.replace(/\/$/, "")}/${encodeURIComponent(accountId)}`
    : "";
}

export function txExplorerUrl(hash) {
  return hash
    ? `${appConfig.explorerTxBase.replace(/\/$/, "")}/${encodeURIComponent(hash)}`
    : "";
}
