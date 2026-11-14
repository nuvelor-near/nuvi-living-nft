import { appConfig } from "../config";
import { viewAccount } from "./rpc";

export const DEMO_FUNDING_TARGET_YOCTO = 1_000_000_000_000_000_000_000_000n;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export function formatNear(yocto, decimals = 3) {
  const amount = BigInt(yocto || "0");
  const unit = 10n ** 24n;
  const whole = amount / unit;
  const fraction = (amount % unit).toString().padStart(24, "0").slice(0, decimals);
  return decimals > 0 ? `${whole}.${fraction}` : String(whole);
}

export async function loadDemoFundingState(accountId) {
  if (!accountId) return { balanceYocto: "0", ready: false };
  const account = await viewAccount(accountId);
  const balanceYocto = String(account?.amount || "0");
  return {
    balanceYocto,
    ready: BigInt(balanceYocto) >= DEMO_FUNDING_TARGET_YOCTO,
  };
}

export async function requestDemoFunding(accountId) {
  if (!accountId) throw new Error("Connect a NEAR Testnet wallet first");

  const response = await fetch(appConfig.sponsorUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "omit",
    body: JSON.stringify({ account_id: accountId }),
  });

  let payload = null;
  try {
    payload = await response.json();
  } catch {
    payload = null;
  }

  if (!response.ok && response.status !== 202) {
    throw new Error(
      payload?.message ||
        payload?.error ||
        `Testnet sponsor failed with HTTP ${response.status}`,
    );
  }

  let funding = await loadDemoFundingState(accountId);
  for (let attempt = 0; attempt < 20 && !funding.ready; attempt += 1) {
    await sleep(750);
    funding = await loadDemoFundingState(accountId);
  }

  if (!funding.ready) {
    throw new Error(
      "Sponsor transaction was submitted, but the 1.0 NEAR balance is not final yet. Retry in a few seconds.",
    );
  }

  return { ...(payload || {}), balanceYocto: funding.balanceYocto, ready: true };
}
