import React, { useEffect, useState } from "react";
import {
  accountExplorerUrl,
  appConfig,
  txExplorerUrl,
} from "./config";
import { formatNuve } from "./near/gameApi";
import { viewFunction } from "./near/rpc";

function external(href, label) {
  return (
    <a href={href} target="_blank" rel="noreferrer">
      {label}
    </a>
  );
}

export default function VerifiedProof() {
  const [proof, setProof] = useState(null);
  const [error, setError] = useState("");

  async function load() {
    setError("");
    try {
      const tokens = await viewFunction(
        appConfig.nftContractId,
        "nft_tokens_for_owner",
        { account_id: appConfig.demoPlayerId, limit: 10 },
      );
      const token = Array.isArray(tokens) ? tokens[0] : null;
      if (!token) throw new Error("Verified E2E token is not available");

      const [living, alignment, playerBalance, treasuryBalance, supply] =
        await Promise.all([
          viewFunction(appConfig.nftContractId, "get_living_state", {
            token_id: String(token.token_id),
          }),
          viewFunction(appConfig.nftContractId, "get_global_alignment"),
          viewFunction(appConfig.nuveContractId, "ft_balance_of", {
            account_id: appConfig.demoPlayerId,
          }),
          viewFunction(appConfig.nuveContractId, "ft_balance_of", {
            account_id: appConfig.treasuryId,
          }),
          viewFunction(appConfig.nuveContractId, "ft_total_supply"),
        ]);

      setProof({
        token,
        living,
        alignment,
        playerBalance,
        treasuryBalance,
        supply,
      });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <section className="panel verified-proof">
      <div className="panel-head">
        <div>
          <span className="eyebrow">Live chain proof · no wallet required</span>
          <h3>Verified Testnet run</h3>
        </div>
        <button className="button ghost" onClick={() => void load()}>
          Refresh proof
        </button>
      </div>

      {error ? (
        <p className="muted">Unable to refresh proof: {error}</p>
      ) : !proof ? (
        <p className="muted">Reading verified Testnet state…</p>
      ) : (
        <>
          <div className="proof-grid">
            <div>
              <span>Token</span>
              <strong>#{proof.token.token_id}</strong>
            </div>
            <div>
              <span>Form</span>
              <strong>{proof.living.form_id}</strong>
            </div>
            <div>
              <span>XP</span>
              <strong>{proof.living.xp}</strong>
            </div>
            <div>
              <span>Work A / B</span>
              <strong>
                {proof.living.work_a_xp} / {proof.living.work_b_xp}
              </strong>
            </div>
            <div>
              <span>Sponsor / Burn</span>
              <strong>
                {formatNuve(proof.living.sponsor_spent)} /{" "}
                {formatNuve(proof.living.burner_spent)}
              </strong>
            </div>
            <div>
              <span>Player NUVE</span>
              <strong>{formatNuve(proof.playerBalance)}</strong>
            </div>
            <div>
              <span>Treasury NUVE</span>
              <strong>{formatNuve(proof.treasuryBalance)}</strong>
            </div>
            <div>
              <span>Total supply</span>
              <strong>{formatNuve(proof.supply)}</strong>
            </div>
          </div>

          <div className="proof-links">
            {external(
              accountExplorerUrl(appConfig.demoPlayerId),
              "E2E player ↗",
            )}
            {external(
              accountExplorerUrl(appConfig.nftContractId),
              "Living NFT contract ↗",
            )}
            {external(
              txExplorerUrl(appConfig.demoMutationTx),
              "Mutation transaction ↗",
            )}
          </div>

          <p className="muted">
            Global Alignment: Treasury{" "}
            {(Number(proof.alignment.treasury_share_bps) / 100).toFixed(2)}% ·
            Burn {(Number(proof.alignment.burn_share_bps) / 100).toFixed(2)}%.
            This panel is read directly from NEAR Testnet.
          </p>
        </>
      )}
    </section>
  );
}
