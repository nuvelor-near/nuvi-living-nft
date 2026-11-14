import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  accountExplorerUrl,
  appConfig,
  contractsConfigured,
  txExplorerUrl,
} from "./config";
import { useWalletConnect } from "./wallet/WalletConnectContext";
import {
  actionReady,
  claimStarterNuve,
  claimStarterNuvi,
  completeLivingAction,
  formatNuve,
  loadAccountState,
  loadTokenState,
  mutateNuvi,
  nsToMs,
  sendNuviAccountNuve,
  startLivingAction,
} from "./near/gameApi";
import { extractTransactionHashes } from "./near/transactions";
import VerifiedProof from "./VerifiedProof";

const ACTIONS = [
  { id: "feed", label: "Feed", detail: "30 sec · reward 1 NUVE" },
  { id: "work_a", label: "Work A", detail: "instant · +1 XP / A affinity" },
  { id: "work_b", label: "Work B", detail: "instant · +1 XP / B affinity" },
];

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function shortId(value, head = 12, tail = 8) {
  if (!value) return "—";
  if (value.length <= head + tail + 3) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

function formLabel(formId) {
  if (formId === "form_a") return "Form A";
  if (formId === "form_b") return "Form B";
  return "Basic";
}

function countdown(readyAtNs, nowMs) {
  if (!readyAtNs) return "—";
  const remaining = Math.max(0, nsToMs(readyAtNs) - nowMs);
  if (remaining <= 0) return "Ready";
  return `${Math.ceil(remaining / 1000)}s`;
}

function Stat({ label, value }) {
  return (
    <div className="stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ExternalLink({ href, children }) {
  if (!href) return children;
  return (
    <a href={href} target="_blank" rel="noreferrer">
      {children}
    </a>
  );
}

export default function App() {
  const {
    connector,
    accountId,
    isLoading: walletLoading,
    error: walletError,
    connect,
    disconnect,
  } = useWalletConnect();

  const [accountState, setAccountState] = useState(null);
  const [tokenState, setTokenState] = useState(null);
  const [selectedTokenId, setSelectedTokenId] = useState("");
  const [route, setRoute] = useState("sponsor");
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [activity, setActivity] = useState([]);
  const [nowMs, setNowMs] = useState(Date.now());
  const [nuviRecipient, setNuviRecipient] = useState("");
  const [nuviAmount, setNuviAmount] = useState("1.0000");

  useEffect(() => {
    const id = window.setInterval(() => setNowMs(Date.now()), 500);
    return () => window.clearInterval(id);
  }, []);


  useEffect(() => {
    if (accountId) setNuviRecipient(accountId);
  }, [accountId]);

  const refresh = useCallback(
    async (preferredTokenId = "") => {
      if (!accountId || !contractsConfigured) {
        setAccountState(null);
        setTokenState(null);
        return;
      }

      const nextAccount = await loadAccountState(accountId);
      const ownedIds = nextAccount.tokens.map((token) => String(token.token_id));
      const current =
        preferredTokenId ||
        (ownedIds.includes(selectedTokenId) ? selectedTokenId : "") ||
        ownedIds[0] ||
        "";

      setAccountState(nextAccount);
      setSelectedTokenId(current);

      if (current) {
        setTokenState(await loadTokenState(current));
      } else {
        setTokenState(null);
      }
    },
    [accountId, selectedTokenId],
  );

  useEffect(() => {
    if (tokenState?.nuviAccount?.status !== "provisioning" || !selectedTokenId) {
      return undefined;
    }
    const id = window.setInterval(() => {
      void refresh(selectedTokenId);
    }, 2500);
    return () => window.clearInterval(id);
  }, [refresh, selectedTokenId, tokenState?.nuviAccount?.status]);

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      setError("");
      if (!accountId || !contractsConfigured) {
        setAccountState(null);
        setTokenState(null);
        return;
      }
      try {
        await refresh();
      } catch (cause) {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      }
    }

    void boot();
    return () => {
      cancelled = true;
    };
  }, [accountId, refresh]);

  async function selectToken(tokenId) {
    setSelectedTokenId(tokenId);
    setError("");
    try {
      setTokenState(await loadTokenState(tokenId));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function runTx(label, operation) {
    if (busy) return;

    setBusy(label);
    setNotice("");
    setError("");

    try {
      const result = await operation();
      const hashes = extractTransactionHashes(result);
      setActivity((items) => [
        ...hashes.map((hash) => ({
          label,
          hash,
          at: new Date().toISOString(),
        })),
        ...items,
      ].slice(0, 8));

      setNotice(
        hashes.length
          ? `${label}: transaction accepted`
          : `${label}: wallet flow completed`,
      );

      await sleep(1_200);
      await refresh();
      await sleep(1_800);
      await refresh();
      return result;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      throw cause;
    } finally {
      setBusy("");
    }
  }

  const token = tokenState?.token;
  const living = tokenState?.living;
  const mutation = tokenState?.mutation;
  const nuviAccount = tokenState?.nuviAccount;

  const activeAction = living?.active_action || null;
  const ready = actionReady(living, nowMs);

  const actionCost = useMemo(() => {
    const config = accountState?.gameConfig;
    return {
      feed: config?.feed_cost || "50000",
      work_a: config?.work_cost || "100000",
      work_b: config?.work_cost || "100000",
    };
  }, [accountState]);

  const imageUrl = token?.metadata?.media || "";
  const global = accountState?.alignment;
  const treasuryPct = Number(global?.treasury_share_bps || 0) / 100;
  const burnPct = Number(global?.burn_share_bps || 0) / 100;

  return (
    <div className="app-shell">
      <header className="topbar">
        <div>
          <div className="eyebrow">NEAR Hackathon 2026 · Testnet</div>
          <h1>NUVI Living NFT</h1>
          <p>One NFT. Persistent identity. Player-shaped evolution.</p>
        </div>

        <div className="wallet-panel">
          {walletLoading ? (
            <span className="status-pill">Wallet loading…</span>
          ) : accountId ? (
            <>
              <ExternalLink href={accountExplorerUrl(accountId)}>
                <span className="account-id">{shortId(accountId)}</span>
              </ExternalLink>
              <button className="button ghost" onClick={() => void disconnect()}>
                Disconnect
              </button>
            </>
          ) : (
            <button className="button primary" onClick={() => void connect()}>
              Connect NEAR wallet
            </button>
          )}
        </div>
      </header>

      {!contractsConfigured && (
        <section className="banner warning">
          <strong>Testnet deployment is the next gate.</strong>
          <span>
            UI is ready for V7 APIs; set VITE_NFT_CONTRACT_ID,
            VITE_NUVE_CONTRACT_ID and VITE_TREASURY_ID after isolated deployment.
          </span>
        </section>
      )}

      {(walletError || error) && (
        <section className="banner error">
          <strong>Action failed</strong>
          <span>{walletError || error}</span>
        </section>
      )}

      {notice && (
        <section className="banner success">
          <strong>Confirmed</strong>
          <span>{notice}</span>
        </section>
      )}

      <main className="dashboard">
        <section className="hero-card">
          <div className="hero-copy">
            <span className="status-pill accent">Living NFT · V7 client</span>
            <h2>Your actions shape the same token.</h2>
            <p>
              Feed, work, choose Treasury or Burn, accumulate XP and economic
              behavior, then mutate the same NEP-171 token into a new form.
            </p>
            <div className="contract-row">
              <ExternalLink href={accountExplorerUrl(appConfig.nftContractId)}>
                NFT: {shortId(appConfig.nftContractId)}
              </ExternalLink>
              <ExternalLink href={accountExplorerUrl(appConfig.nuveContractId)}>
                NUVE: {shortId(appConfig.nuveContractId)}
              </ExternalLink>
            </div>
          </div>
          <div className="alignment-mini">
            <span>Global Alignment</span>
            <strong>{formatNuve(global?.global_total || "0")} NUVE</strong>
            <small>
              Treasury {treasuryPct.toFixed(2)}% · Burn {burnPct.toFixed(2)}%
            </small>
          </div>
        </section>

        {!accountId ? (
          <section className="empty-state">
            <div className="orb" />
            <h2>Connect a Testnet wallet to start.</h2>
            <p>
              The demo uses the same wallet connector path proven in NUVELOR,
              isolated to NEAR Testnet.
            </p>
            <button className="button primary large" onClick={() => void connect()}>
              Connect wallet
            </button>
          </section>
        ) : (
          <>
            <section className="grid two">
              <article className="panel">
                <div className="panel-head">
                  <div>
                    <span className="eyebrow">Onboarding</span>
                    <h3>Starter assets</h3>
                  </div>
                  <span className="nuve-balance">
                    {formatNuve(accountState?.nuveBalance || "0")} NUVE
                  </span>
                </div>

                <div className="starter-actions">
                  <button
                    className="button primary"
                    disabled={
                      !contractsConfigured ||
                      accountState?.nftClaimUsed ||
                      Boolean(busy)
                    }
                    onClick={() =>
                      void runTx("Claim Starter NUVI", () =>
                        claimStarterNuvi(connector),
                      )
                    }
                  >
                    {accountState?.nftClaimUsed
                      ? "Starter NUVI claimed"
                      : busy === "Claim Starter NUVI"
                        ? "Claiming…"
                        : "Claim Starter NUVI"}
                  </button>

                  <button
                    className="button secondary"
                    disabled={
                      !contractsConfigured ||
                      accountState?.nuveClaimUsed ||
                      Boolean(busy)
                    }
                    onClick={() =>
                      void runTx("Claim 100 NUVE", () =>
                        claimStarterNuve(connector, accountId),
                      )
                    }
                  >
                    {accountState?.nuveClaimUsed
                      ? "100 NUVE claimed"
                      : busy === "Claim 100 NUVE"
                        ? "Claiming…"
                        : accountState?.nuveStorage
                          ? "Claim 100 NUVE"
                          : "Register + claim 100 NUVE"}
                  </button>
                </div>
              </article>

              <article className="panel">
                <div className="panel-head">
                  <div>
                    <span className="eyebrow">Community signal</span>
                    <h3>Global NUVE Alignment</h3>
                  </div>
                </div>
                <div className="alignment-track">
                  <div
                    className="alignment-treasury"
                    style={{ width: `${treasuryPct}%` }}
                  />
                  <div
                    className="alignment-burn"
                    style={{ width: `${burnPct}%` }}
                  />
                </div>
                <div className="alignment-legend">
                  <span>
                    Treasury <strong>{formatNuve(global?.treasury_total || "0")}</strong>
                  </span>
                  <span>
                    Burn <strong>{formatNuve(global?.burn_total || "0")}</strong>
                  </span>
                </div>
              </article>
            </section>

            {accountState?.tokens?.length > 0 ? (
              <>
                <section className="token-picker">
                  <span>Owned NUVI</span>
                  <select
                    value={selectedTokenId}
                    onChange={(event) => void selectToken(event.target.value)}
                  >
                    {accountState.tokens.map((item) => (
                      <option key={item.token_id} value={item.token_id}>
                        #{item.token_id} · {item.metadata?.title || "NUVI"}
                      </option>
                    ))}
                  </select>
                </section>

                <section className="grid nft-layout">
                  <article className="panel nft-card">
                    <div className="media-frame">
                      <div className="visual-fallback">
                        <span>NUVI</span>
                        <strong>{formLabel(living?.form_id)}</strong>
                      </div>
                      {imageUrl && (
                        <img
                          src={imageUrl}
                          alt={`NUVI ${formLabel(living?.form_id)}`}
                          onError={(event) => {
                            event.currentTarget.style.display = "none";
                          }}
                        />
                      )}
                      <span className="form-badge">
                        {formLabel(living?.form_id)}
                      </span>
                    </div>

                    <div className="nft-title">
                      <div>
                        <span className="eyebrow">Token #{selectedTokenId}</span>
                        <h3>{token?.metadata?.title || "NUVI Starter"}</h3>
                      </div>
                      <span className="status-pill">
                        mutation {living?.mutation_count || 0}/1
                      </span>
                    </div>

                    <div className="stats-grid">
                      <Stat label="Level" value={living?.level ?? "—"} />
                      <Stat label="XP" value={living?.xp ?? "—"} />
                      <Stat label="Work A" value={living?.work_a_xp ?? "—"} />
                      <Stat label="Work B" value={living?.work_b_xp ?? "—"} />
                      <Stat
                        label="Sponsor"
                        value={`${formatNuve(living?.sponsor_spent || "0")} NUVE`}
                      />
                      <Stat
                        label="Burner"
                        value={`${formatNuve(living?.burner_spent || "0")} NUVE`}
                      />
                    </div>
                  </article>

                  <div className="stack">
                    <article className="panel">
                      <div className="panel-head">
                        <div>
                          <span className="eyebrow">Economic choice</span>
                          <h3>Feed / Work</h3>
                        </div>
                        {activeAction && (
                          <span className={`status-pill ${ready ? "ready" : ""}`}>
                            {ready
                              ? "Ready"
                              : countdown(living?.ready_at_ns, nowMs)}
                          </span>
                        )}
                      </div>

                      <div className="route-switch">
                        <button
                          className={route === "sponsor" ? "active" : ""}
                          disabled={Boolean(activeAction)}
                          onClick={() => setRoute("sponsor")}
                        >
                          Treasury / Sponsor
                        </button>
                        <button
                          className={route === "burn" ? "active" : ""}
                          disabled={Boolean(activeAction)}
                          onClick={() => setRoute("burn")}
                        >
                          Burn / Burner
                        </button>
                      </div>

                      {activeAction ? (
                        <div className="active-action">
                          <div>
                            <span>Active action</span>
                            <strong>{activeAction.replace("_", " ")}</strong>
                          </div>
                          <div>
                            <span>Completion</span>
                            <strong>{countdown(living?.ready_at_ns, nowMs)}</strong>
                          </div>
                          <button
                            className="button primary"
                            disabled={!ready || Boolean(busy)}
                            onClick={() =>
                              void runTx("Complete Living action", () =>
                                completeLivingAction(connector, selectedTokenId),
                              )
                            }
                          >
                            {ready ? "Complete action" : "Timer running"}
                          </button>
                        </div>
                      ) : (
                        <div className="action-list">
                          {ACTIONS.map((action) => (
                            <button
                              key={action.id}
                              className="action-card"
                              disabled={Boolean(busy)}
                              onClick={() =>
                                void runTx(`${action.label} / ${route}`, () =>
                                  startLivingAction(connector, {
                                    tokenId: selectedTokenId,
                                    action: action.id,
                                    route,
                                  }),
                                )
                              }
                            >
                              <span>
                                <strong>{action.label}</strong>
                                <small>{action.detail}</small>
                              </span>
                              <b>{formatNuve(actionCost[action.id])} NUVE</b>
                            </button>
                          ))}
                        </div>
                      )}
                    </article>

                    <article className="panel">
                      <div className="panel-head">
                        <div>
                          <span className="eyebrow">Evolution</span>
                          <h3>Mutation preview</h3>
                        </div>
                        <span className="status-pill">
                          XP {mutation?.xp ?? 0}/{mutation?.threshold_xp ?? 5}
                        </span>
                      </div>

                      <div className="probability">
                        <div>
                          <span>Form A</span>
                          <strong>
                            {((mutation?.form_a_probability_bps || 0) / 100).toFixed(2)}%
                          </strong>
                        </div>
                        <div className="probability-track">
                          <span
                            style={{
                              width: `${(mutation?.form_a_probability_bps || 0) / 100}%`,
                            }}
                          />
                        </div>
                        <small>
                          Work A {mutation?.work_a_xp || 0} · Sponsor level{" "}
                          {mutation?.sponsor_level || 0}
                        </small>
                      </div>

                      <div className="probability">
                        <div>
                          <span>Form B</span>
                          <strong>
                            {((mutation?.form_b_probability_bps || 0) / 100).toFixed(2)}%
                          </strong>
                        </div>
                        <div className="probability-track b">
                          <span
                            style={{
                              width: `${(mutation?.form_b_probability_bps || 0) / 100}%`,
                            }}
                          />
                        </div>
                        <small>
                          Work B {mutation?.work_b_xp || 0} · Burner level{" "}
                          {mutation?.burner_level || 0}
                        </small>
                      </div>

                      <button
                        className="button mutation"
                        disabled={!mutation?.eligible || Boolean(busy)}
                        onClick={() =>
                          void runTx("Mutate NUVI", () =>
                            mutateNuvi(connector, selectedTokenId),
                          )
                        }
                      >
                        {living?.mutation_count
                          ? "MVP mutation complete"
                          : mutation?.eligible
                            ? "Mutate NUVI"
                            : `Complete ${mutation?.threshold_xp ?? 1} Work cycle${(mutation?.threshold_xp ?? 1) === 1 ? "" : "s"} to mutate`}
                      </button>
                    </article>

                    <article className="panel nuvi-account-panel">
                      <div className="panel-head">
                        <div>
                          <span className="eyebrow">NFT-controlled account</span>
                          <h3>NUVI Account</h3>
                        </div>
                        <span className={`status-pill ${nuviAccount?.status === "active" ? "ready" : ""}`}>
                          {nuviAccount?.status === "not_created"
                            ? "Not created"
                            : nuviAccount?.status === "provisioning"
                              ? "Provisioning"
                              : nuviAccount?.status === "active"
                                ? "Active"
                                : nuviAccount?.status === "failed"
                                  ? "Failed"
                                  : "—"}
                        </span>
                      </div>

                      <div className="account-address">
                        <span>Smart account</span>
                        <ExternalLink href={accountExplorerUrl(nuviAccount?.accountId)}>
                          <strong>{nuviAccount?.accountId || "—"}</strong>
                        </ExternalLink>
                      </div>

                      <div className="stats-grid account-proof-grid">
                        <Stat label="Bound NFT" value={`NUVI #${selectedTokenId}`} />
                        <Stat label="Controller" value={token?.owner_id || "—"} />
                        <Stat
                          label="Access keys"
                          value={
                            Array.isArray(nuviAccount?.accessKeys)
                              ? `${nuviAccount.accessKeys.length} · ${nuviAccount.accessKeys.length === 0 ? "Locked" : "Unexpected"}`
                              : "—"
                          }
                        />
                        <Stat
                          label="Executor"
                          value={nuviAccount?.verified ? "Deployed · verified" : nuviAccount?.executor ? "Deployed" : "—"}
                        />
                        <Stat label="NUVE balance" value={`${formatNuve(nuviAccount?.nuveBalance || "0")} NUVE`} />
                        <Stat label="NFT metadata" value={nuviAccount?.metadataStatus || "—"} />
                      </div>

                      {nuviAccount?.status === "failed" && (
                        <div className="banner error account-failure">
                          Provisioning failed{nuviAccount?.failurePhase ? `: ${nuviAccount.failurePhase}` : ""}.
                        </div>
                      )}

                      {nuviAccount?.status === "active" && (
                        <div className="nuvi-transfer">
                          <button
                            className="button primary"
                            disabled={Boolean(busy) || !accountId}
                            onClick={() =>
                              void runTx("NUVI Account · Send 1 NUVE", () =>
                                sendNuviAccountNuve(connector, {
                                  nuviAccountId: nuviAccount.accountId,
                                  receiverId: accountId,
                                  amountNuve: "1.0000",
                                }),
                              )
                            }
                          >
                            Send 1 NUVE to my wallet
                          </button>

                          <details className="advanced-transfer">
                            <summary>Custom Testnet transfer</summary>
                            <div className="transfer-fields">
                              <label>
                                Recipient
                                <input
                                  value={nuviRecipient}
                                  onChange={(event) => setNuviRecipient(event.target.value)}
                                  placeholder="account.testnet"
                                />
                              </label>
                              <label>
                                Amount (NUVE)
                                <input
                                  value={nuviAmount}
                                  onChange={(event) => setNuviAmount(event.target.value)}
                                  inputMode="decimal"
                                />
                              </label>
                              <button
                                className="button secondary"
                                disabled={Boolean(busy) || !nuviRecipient || !nuviAmount}
                                onClick={() =>
                                  void runTx("NUVI Account · Custom transfer", () =>
                                    sendNuviAccountNuve(connector, {
                                      nuviAccountId: nuviAccount.accountId,
                                      receiverId: nuviRecipient.trim(),
                                      amountNuve: nuviAmount,
                                    }),
                                  )
                                }
                              >
                                Send from NUVI Account
                              </button>
                            </div>
                          </details>
                        </div>
                      )}
                    </article>
                  </div>
                </section>
              </>
            ) : (
              <section className="empty-state compact">
                <h2>No NUVI in this wallet yet.</h2>
                <p>Claim the one-time starter NFT to enter the gameplay loop.</p>
              </section>
            )}

            <section className="panel evidence">
              <div className="panel-head">
                <div>
                  <span className="eyebrow">Judge evidence</span>
                  <h3>Recent transactions</h3>
                </div>
                <button
                  className="button ghost"
                  disabled={Boolean(busy)}
                  onClick={() => void refresh()}
                >
                  Refresh state
                </button>
              </div>

              {activity.length ? (
                <div className="activity-list">
                  {activity.map((item) => (
                    <ExternalLink
                      href={txExplorerUrl(item.hash)}
                      key={`${item.hash}-${item.at}`}
                    >
                      <span>{item.label}</span>
                      <code>{shortId(item.hash, 14, 10)}</code>
                    </ExternalLink>
                  ))}
                </div>
              ) : (
                <p className="muted">
                  Transaction hashes from this session will appear here with Testnet explorer links.
                </p>
              )}
            </section>
          </>
        )}

        <VerifiedProof />
      </main>

      <footer>
        <span>NUVI Living NFT · isolated Hackathon Testnet demo</span>
        <ExternalLink href={accountExplorerUrl(appConfig.treasuryId)}>
          Treasury {shortId(appConfig.treasuryId)}
        </ExternalLink>
      </footer>
    </div>
  );
}
