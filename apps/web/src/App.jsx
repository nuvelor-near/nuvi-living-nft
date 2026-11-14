import React, { useCallback, useEffect, useMemo, useState } from "react";
import { accountExplorerUrl, contractsConfigured, txExplorerUrl } from "./config";
import { useWalletConnect } from "./wallet/WalletConnectContext";
import {
  claimStarterNuve,
  claimStarterNuvi,
  formatNuve,
  loadAccountState,
  loadTokenState,
  mutateNuvi,
  sendNuviAccountNuve,
  startLivingAction,
} from "./near/gameApi";
import { extractTransactionHashes } from "./near/transactions";
import {
  DEMO_FUNDING_TARGET_YOCTO,
  formatNear,
  loadDemoFundingState,
  requestDemoFunding,
} from "./near/sponsor";
import { getNuviDemoCopy } from "./demoCopyV2";
import {
  emitWalletDiagnostic,
  isMobileBrowser,
  primeMeteorWebWindow,
  releasePrimedMeteorWebWindow,
} from "./wallet/diagnostics";

const PHASES = ["connect", "fund", "nft", "nuve", "work", "mutate", "control", "proof"];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function shortId(value, head = 12, tail = 8) {
  if (!value) return "—";
  if (value.length <= head + tail + 3) return value;
  return value.slice(0, head) + "…" + value.slice(-tail);
}

function formatCompactNuve(value) {
  const formatted = formatNuve(value);
  if (!formatted.includes(".")) return formatted;
  return formatted.replace(/0+$/, "").replace(/\.$/, "");
}

function formLabel(formId, t) {
  if (formId === "form_a") return t.formA;
  if (formId === "form_b") return t.formB;
  return t.basic;
}

function formMediaUrl(formId) {
  if (formId === "form_a") return "/assets/hackathon/nuvi/form_a.webp";
  if (formId === "form_b") return "/assets/hackathon/nuvi/form_b.webp";
  return "/assets/hackathon/nuvi/basic.webp";
}

function ExternalLink({ href, children, className = "" }) {
  if (!href) return children;
  return (
    <a className={className} href={href} target="_blank" rel="noreferrer">
      {children}
    </a>
  );
}

function NuviVisual({ imageUrl, form, tokenId, ghost = false }) {
  return (
    <div className={"nuvi-demo-visual" + (ghost ? " is-ghost" : "")}>
      <div className="nuvi-demo-orbit" aria-hidden="true" />
      <img src={imageUrl} alt={"NUVI " + form} />
      <div className="nuvi-demo-visual-meta">
        <strong>{tokenId ? "NUVI #" + tokenId : "NUVI"}</strong>
        <span>{form}</span>
      </div>
    </div>
  );
}

function Progress({ phase, t }) {
  const current = Math.max(0, PHASES.indexOf(phase));
  return (
    <div className="nuvi-demo-progress" aria-label={t.progressLabel}>
      {PHASES.map((item, index) => (
        <span
          key={item}
          className={(index < current ? "done " : "") + (index === current ? "active" : "")}
          title={t.steps[item]}
        />
      ))}
    </div>
  );
}

export default function DemoApp({ locale = "en" }) {
  const {
    connector,
    accountId,
    walletId,
    isLoading: walletLoading,
    error: walletError,
    connect,
    disconnect,
  } = useWalletConnect();

  const t = useMemo(() => getNuviDemoCopy(locale), [locale]);
  const [accountState, setAccountState] = useState(null);
  const [tokenState, setTokenState] = useState(null);
  const [selectedTokenId, setSelectedTokenId] = useState("");
  const [demoNearBalance, setDemoNearBalance] = useState("0");
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [proofTxHash, setProofTxHash] = useState("");

  const refresh = useCallback(
    async (preferredTokenId = "") => {
      if (!accountId || !contractsConfigured) {
        setAccountState(null);
        setTokenState(null);
        setSelectedTokenId("");
        return;
      }

      const [nextAccount, fundingState] = await Promise.all([
        loadAccountState(accountId),
        loadDemoFundingState(accountId),
      ]);

      setAccountState(nextAccount);
      setDemoNearBalance(fundingState.balanceYocto);

      const ownedIds = nextAccount.tokens.map((item) => String(item.token_id));
      const nextTokenId =
        preferredTokenId ||
        (ownedIds.includes(selectedTokenId) ? selectedTokenId : "") ||
        ownedIds[0] ||
        "";

      setSelectedTokenId(nextTokenId);
      setTokenState(nextTokenId ? await loadTokenState(nextTokenId) : null);
    },
    [accountId, selectedTokenId],
  );

  useEffect(() => {
    let cancelled = false;
    async function boot() {
      if (!accountId) {
        setAccountState(null);
        setTokenState(null);
        setSelectedTokenId("");
        return;
      }
      try {
        await refresh();
      } catch (cause) {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      }
    }
    void boot();
    return () => {
      cancelled = true;
    };
  }, [accountId, refresh]);

  useEffect(() => {
    if (!selectedTokenId || tokenState?.nuviAccount?.status !== "provisioning") return undefined;
    const id = window.setInterval(() => void refresh(selectedTokenId), 1800);
    return () => window.clearInterval(id);
  }, [refresh, selectedTokenId, tokenState?.nuviAccount?.status]);

  async function handleDisconnect() {
    if (busy) return;
    setBusy("disconnect");
    setError("");
    setNotice("");
    try {
      await disconnect();
      setProofTxHash("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy("");
    }
  }

  async function runTx(label, operation) {
    if (busy) return null;

    const meteorMobile = walletId === "meteor-wallet" && isMobileBrowser();
    const primed = primeMeteorWebWindow(walletId, label);
    if (meteorMobile && !primed) {
      emitWalletDiagnostic("meteor-web-prime-required", { label });
      setError(t.preparingSignatureBlocked);
      return null;
    }

    setBusy(label);
    setError("");
    setNotice("");
    emitWalletDiagnostic("ui-tx-start", {
      label,
      walletId: walletId || "unknown",
      meteorMobile,
      signingWindowPrimed: primed,
    });

    try {
      const result = await operation();
      const hashes = extractTransactionHashes(result);
      setNotice(hashes.length ? t.txAccepted : t.verifyingState);
      await sleep(900);
      await refresh();
      await sleep(1100);
      await refresh();
      return result;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    } finally {
      releasePrimedMeteorWebWindow();
      setBusy("");
    }
  }

  async function fundWallet() {
    if (!accountId || busy) return;
    setBusy("fund");
    setError("");
    setNotice("");
    try {
      const result = await requestDemoFunding(accountId);
      setDemoNearBalance(result.balanceYocto);
      setNotice(t.demoGasFunded);
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy("");
    }
  }

  const token = [REDACTED] || null;
  const living = tokenState?.living || null;
  const mutation = tokenState?.mutation || null;
  const nuviAccount = tokenState?.nuviAccount || null;
  const imageUrl = token?.metadata?.media || formMediaUrl(living?.form_id);
  const form = formLabel(living?.form_id, t);
  const balanceYocto = BigInt(demoNearBalance || "0");
  const gasReady = balanceYocto >= DEMO_FUNDING_TARGET_YOCTO;
  const sponsorEligible =
    Boolean(accountState) && !accountState.nftClaimUsed && !accountState.nuveClaimUsed;
  const hasNuvi = Boolean(token && selectedTokenId);
  const mutationCount = Number(living?.mutation_count || 0);
  const threshold = Number(
    mutation?.threshold_xp ?? accountState?.gameConfig?.mutation_threshold_xp ?? 1,
  );
  const xp = Number(living?.xp || 0);
  const mutationReady = Boolean(mutation?.eligible || xp >= threshold);
  const accountActive = nuviAccount?.status === "active" && nuviAccount?.verified;

  let phase = "connect";
  if (accountId && !accountState) phase = "loading";
  else if (accountId && !gasReady && sponsorEligible) phase = "fund";
  else if (accountId && !hasNuvi) phase = "nft";
  else if (accountId && !accountState?.nuveClaimUsed) phase = "nuve";
  else if (accountId && mutationCount === 0 && !mutationReady) phase = "work";
  else if (accountId && mutationCount === 0) phase = "mutate";
  else if (accountId && !accountActive) phase = "control";
  else if (accountId && proofTxHash) phase = "proof";
  else if (accountId) phase = "control";

  const phaseForProgress = phase === "loading" ? "nft" : phase;
  const currentIndex = Math.max(0, PHASES.indexOf(phaseForProgress));
  const currentStep = currentIndex + 1;

  async function chooseWork(action) {
    const route = action === "work_a" ? "sponsor" : "burn";
    await runTx(action === "work_a" ? t.workA : t.workB, () =>
      startLivingAction(connector, {
        tokenId: selectedTokenId,
        action,
        route,
      }),
    );
  }

  async function proveControl() {
    const result = await runTx(t.nuviAccountSend1, () =>
      sendNuviAccountNuve(connector, {
        nuviAccountId: nuviAccount.accountId,
        receiverId: accountId,
        amountNuve: "1.0000",
      }),
    );
    const hashes = extractTransactionHashes(result);
    if (hashes.length) setProofTxHash(hashes[hashes.length - 1]);
  }

  function PhoneHeader() {
    return (
      <>
        <div className="nuvi-demo-phone-head">
          <span className="nuvi-demo-step-count">
            {t.stepLabel} {currentStep}/{PHASES.length}
          </span>
          {accountId ? (
            <ExternalLink href={accountExplorerUrl(accountId)} className="nuvi-demo-wallet connected">
              {shortId(accountId, 10, 6)}
            </ExternalLink>
          ) : (
            <span className="nuvi-demo-wallet">{t.testnet}</span>
          )}
        </div>
        <Progress phase={phaseForProgress} t={t} />
      </>
    );
  }

  function StatusArea() {
    if (walletError || error) {
      return (
        <div className="nuvi-demo-status error">
          <strong>{t.actionFailed}</strong>
          <span>{walletError || error}</span>
        </div>
      );
    }
    if (notice) {
      return (
        <div className="nuvi-demo-status success">
          <strong>{t.confirmed}</strong>
          <span>{notice}</span>
        </div>
      );
    }
    return null;
  }

  return (
    <div className="nuvi-demo-app">
      <div className="nuvi-demo-stage">
        <section className="nuvi-demo-phone" aria-live="polite">
          <PhoneHeader />
          <StatusArea />

          {!contractsConfigured ? (
            <div className="nuvi-demo-screen centered">
              <span className="nuvi-demo-kicker">{t.nearHackathon}</span>
              <h3>{t.deploymentGate}</h3>
              <p>{t.deploymentInstruction}</p>
            </div>
          ) : phase === "loading" ? (
            <div className="nuvi-demo-screen centered">
              <div className="nuvi-demo-spinner" aria-hidden="true" />
              <h3>{t.loadingStateTitle}</h3>
              <p>{t.loadingStateDescription}</p>
            </div>
          ) : phase === "connect" ? (
            <div className="nuvi-demo-screen centered">
              <NuviVisual
                imageUrl="/assets/hackathon/nuvi/basic.webp"
                form={t.basic}
                tokenId=""
                ghost
              />
              <span className="nuvi-demo-kicker">{t.steps.connect}</span>
              <h3>{t.connectTitle}</h3>
              <p>{t.connectDescription}</p>
              <button
                className="nuvi-demo-primary"
                disabled={walletLoading}
                onClick={() => void connect()}
              >
                {walletLoading ? t.walletLoading : t.connectWallet}
              </button>
              <small>{t.testnetOnly}</small>
            </div>
          ) : phase === "fund" ? (
            <div className="nuvi-demo-screen centered">
              <div className="nuvi-demo-gas-ring">
                <span>{t.balance}</span>
                <strong>{formatNear(demoNearBalance)} NEAR</strong>
              </div>
              <span className="nuvi-demo-kicker">{t.steps.fund}</span>
              <h3>{t.sponsorTitle}</h3>
              <button
                className="nuvi-demo-primary"
                disabled={Boolean(busy)}
                onClick={() => void fundWallet()}
              >
                {busy === "fund" ? t.fundingDemoGas : t.getDemoGas}
              </button>
              <small className="nuvi-demo-sponsor-note">
                {t.sponsorDescription} {t.sponsorSafety}
              </small>
            </div>
          ) : phase === "nft" ? (
            <div className="nuvi-demo-screen centered">
              <NuviVisual
                imageUrl="/assets/hackathon/nuvi/basic.webp"
                form={t.basic}
                tokenId=""
                ghost
              />
              <span className="nuvi-demo-kicker">{t.steps.nft}</span>
              <h3>{t.claimNftTitle}</h3>
              <p>{t.claimNftDescription}</p>
              <button
                className="nuvi-demo-primary"
                disabled={Boolean(busy)}
                onClick={() => void runTx(t.claimStarter, () => claimStarterNuvi(connector))}
              >
                {busy ? t.signing : t.claimStarter}
              </button>
              <small>{t.claimNftHint}</small>
            </div>
          ) : phase === "nuve" ? (
            <div className="nuvi-demo-screen centered">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <span className="nuvi-demo-kicker">{t.steps.nuve}</span>
              <h3>{t.claimNuveTitle}</h3>
              <p>{t.claimNuveDescription}</p>
              <div className="nuvi-demo-fact-row">
                <span>{t.owner}</span>
                <strong>{shortId(token?.owner_id || accountId, 14, 8)}</strong>
              </div>
              <button
                className="nuvi-demo-primary"
                disabled={Boolean(busy)}
                onClick={() =>
                  void runTx(t.claimNuve, () => claimStarterNuve(connector, accountId))
                }
              >
                {busy ? t.signing : t.claimNuve}
              </button>
            </div>
          ) : phase === "work" ? (
            <div className="nuvi-demo-screen">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <span className="nuvi-demo-kicker">{t.steps.work}</span>
              <h3>{t.workChoiceTitle}</h3>
              <p>{t.workChoiceDescription}</p>
              <div className="nuvi-demo-choice-grid">
                <button
                  className="nuvi-demo-choice a"
                  aria-label={t.workA + ": " + t.workAChoice}
                  disabled={Boolean(busy)}
                  onClick={() => void chooseWork("work_a")}
                >
                  <span className="nuvi-demo-pill blue" aria-hidden="true"><i /></span>
                  <span className="nuvi-demo-choice-action">{t.tapToChoose}</span>
                  <small className="nuvi-demo-choice-cost">
                    {formatCompactNuve(accountState?.gameConfig?.work_cost || "100000")} NUVE
                  </small>
                </button>
                <button
                  className="nuvi-demo-choice b"
                  aria-label={t.workB + ": " + t.workBChoice}
                  disabled={Boolean(busy)}
                  onClick={() => void chooseWork("work_b")}
                >
                  <span className="nuvi-demo-pill red" aria-hidden="true"><i /></span>
                  <span className="nuvi-demo-choice-action">{t.tapToChoose}</span>
                  <small className="nuvi-demo-choice-cost">
                    {formatCompactNuve(accountState?.gameConfig?.work_cost || "100000")} NUVE
                  </small>
                </button>
              </div>
              <small className="nuvi-demo-note">{t.workRandomHint}</small>
            </div>
          ) : phase === "mutate" ? (
            <div className="nuvi-demo-screen">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <span className="nuvi-demo-kicker">{t.steps.mutate}</span>
              <h3>{t.mutationStepTitle}</h3>
              <p>{t.mutationStepDescription}</p>
              <div className="nuvi-demo-probabilities">
                <div>
                  <span>{t.formA}</span>
                  <strong>
                    {((mutation?.form_a_probability_bps || 0) / 100).toFixed(2)}%
                  </strong>
                </div>
                <div>
                  <span>{t.formB}</span>
                  <strong>
                    {((mutation?.form_b_probability_bps || 0) / 100).toFixed(2)}%
                  </strong>
                </div>
              </div>
              <div className="nuvi-demo-fact-row">
                <span>{t.xp}</span>
                <strong>{xp}/{threshold}</strong>
              </div>
              <button
                className="nuvi-demo-primary mutation"
                disabled={!mutationReady || Boolean(busy)}
                onClick={() =>
                  void runTx(t.mutateNuvi, () => mutateNuvi(connector, selectedTokenId))
                }
              >
                {busy ? t.signing : t.mutateNuvi}
              </button>
              <small className="nuvi-demo-note">{t.mutationRandomHint}</small>
            </div>
          ) : phase === "control" && !accountActive ? (
            <div className="nuvi-demo-screen centered">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <div className="nuvi-demo-spinner" aria-hidden="true" />
              <span className="nuvi-demo-kicker">{t.steps.control}</span>
              <h3>{t.provisionTitle}</h3>
              <p>{t.provisionDescription}</p>
              <div className="nuvi-demo-proof-box">
                <span>{t.smartAccount}</span>
                <strong>{shortId(nuviAccount?.accountId || nuviAccount?.expectedId, 18, 10)}</strong>
              </div>
              <small>{t.provisionHint}</small>
            </div>
          ) : phase === "control" ? (
            <div className="nuvi-demo-screen">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <span className="nuvi-demo-kicker">{t.steps.control}</span>
              <h3>{t.controlTitle}</h3>
              <p>{t.controlDescription}</p>
              <div className="nuvi-demo-proof-grid">
                <div><span>{t.smartAccount}</span><strong>{shortId(nuviAccount?.accountId, 15, 8)}</strong></div>
                <div><span>{t.nuviAccountAccessKeys}</span><strong>{Array.isArray(nuviAccount?.accessKeys) ? nuviAccount.accessKeys.length : "—"}</strong></div>
                <div><span>{t.controller}</span><strong>{shortId(token?.owner_id, 12, 7)}</strong></div>
                <div><span>{t.nuviAccountBalance}</span><strong>{formatNuve(nuviAccount?.nuveBalance || "0")} NUVE</strong></div>
              </div>
              <button
                className="nuvi-demo-primary"
                disabled={Boolean(busy) || !accountActive}
                onClick={() => void proveControl()}
              >
                {busy ? t.signing : t.nuviAccountSend1}
              </button>
              <small className="nuvi-demo-note">{t.controlHint}</small>
            </div>
          ) : (
            <div className="nuvi-demo-screen centered proof">
              <NuviVisual imageUrl={imageUrl} form={form} tokenId={selectedTokenId} />
              <span className="nuvi-demo-success-mark" aria-hidden="true">✓</span>
              <span className="nuvi-demo-kicker">{t.steps.proof}</span>
              <h3>{t.proofCompleteTitle}</h3>
              <p>{t.proofCompleteDescription}</p>
              <div className="nuvi-demo-proof-grid compact">
                <div><span>{t.nftIdentity}</span><strong>#{selectedTokenId}</strong></div>
                <div><span>{t.nuviAccountAccessKeys}</span><strong>0</strong></div>
                <div><span>{t.controller}</span><strong>{shortId(token?.owner_id, 12, 7)}</strong></div>
                <div><span>{t.execution}</span><strong>{t.verified}</strong></div>
              </div>
              <div className="nuvi-demo-actions">
                <ExternalLink className="nuvi-demo-secondary" href={txExplorerUrl(proofTxHash)}>
                  {t.viewTransaction}
                </ExternalLink>
                <ExternalLink className="nuvi-demo-secondary" href={accountExplorerUrl(nuviAccount?.accountId)}>
                  {t.viewAccount}
                </ExternalLink>
              </div>
              <button
                className="nuvi-demo-link-button"
                disabled={Boolean(busy)}
                onClick={() => void handleDisconnect()}
              >
                {t.tryAnotherWallet}
              </button>
            </div>
          )}

          {accountId && phase !== "proof" && phase !== "fund" && (
            <div className="nuvi-demo-phone-foot">
              <span>{t.balance}: {formatNear(demoNearBalance)} NEAR</span>
              {hasNuvi && <span>NUVI #{selectedTokenId}</span>}
              {accountState?.nuveClaimUsed && (
                <span>{formatNuve(accountState?.nuveBalance || "0")} NUVE</span>
              )}
            </div>
          )}
        </section>
      </div>

    </div>
  );
}
