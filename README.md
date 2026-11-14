# NUVI Living NFT

**NEAR Hackathon 2026 — public development repository**

NUVI is a player-shaped Living NFT: the same NEP-171 token keeps its identity while verified on-chain actions shape its experience, economic behavior, form, media, mutation path, and control of a separate keyless NEAR account.

> The player does not collect the character. The player creates who the character becomes.

## Live demo

**Interactive Testnet demo:** https://nuvelor.gg/en/near-hackathon-2026/nuvi-living-nft/

Localized versions are also available under `/uk/`, `/de/`, and `/ru/`.

The demo is isolated from NUVELOR Mainnet/production contracts. Wallet-signed actions use NEAR Testnet only.

## 60-second judge path

The fastest interactive proof now starts from a fresh Testnet wallet without leaving the demo:

1. Connect a NEAR Testnet wallet.
2. Press **Get demo gas**. The bounded sponsor tops a fresh account up to 2.0 NEAR.
3. Claim Starter NUVI and 100 NUVE.
4. Run one Work A/B action. Work settles immediately and awards 1 XP.
5. Mutate the same NFT; mutation provisions its deterministic keyless NUVI Account.
6. Send 1.0000 NUVE from that NUVI Account back to the connected wallet.

Independent machine evidence for this exact fresh-wallet path is deployments/judge-cycle-e2e-testnet.json. Token #8 completed the cycle and its child account has 0 access keys.

The shortest read-only proof still does not require trusting the UI:

1. Open the live demo and inspect the **Verified Testnet E2E** section.
2. Open Living NFT token `#1` on Testnet and read `metadata.extra.nuvi_account_id`.
3. Open `nuvi-1.nuvi-hack26.nvlborc70517034832.testnet`.
4. Verify the child has deployed code and **0 access keys**.
5. Inspect `deployments/smart-account-e2e-testnet.json`:
   - current NFT owner sent NUVE from the child account;
   - a non-owner was rejected;
   - the NFT was transferred to another account;
   - the previous owner was then rejected;
   - the new NFT owner successfully sent NUVE from the **same child account**;
   - the NFT was restored to the original E2E owner after the proof.

That is the core primitive: **NFT ownership is the authorization root for a real keyless NEAR account.**

## Hackathon tracks

Core:

- **NFTs That Do Something**
- **Games With Real Economies**
- **Open Innovation**

Conditional after implementation and acceptance of the Global NUVE community influence mechanic:

- **Memecoins With Mechanics**

Stretch only after the Testnet core passes acceptance:

- Mainnet deployment
- NEAR Intents

## Current status

Implemented and accepted on Testnet:

- LivingState foundation for persistent player-shaped NFT identity;
- isolated NUVE demo FT with one-time starter claim and bounded idempotent game rewards;
- Living Settlement V1:
  `ft_transfer_call → Sponsor/Burn route → callback`; Feed starts its timer, while Work A/B resolve in the same signed flow;
- Living Actions V1:
  one Work A/B signature routes NUVE and immediately awards XP + affinity; Feed keeps its timer and mints a retry-safe 1 NUVE reward on completion;
- Mutation V1:
  the Hackathon demo uses `WORK_CYCLES_PER_MUTATION = 1`; after one completed Work cycle the current owner gets an on-chain Form A/Form B probability preview and may perform the single MVP mutation without changing `token_id`. The production cycle count is intentionally deferred to the final game economy;
- Global Alignment Base:
  successful Treasury/Burn routes accumulate global NUVE totals and canonical basis-point shares;
- Starter NUVI / contract state V7:
  each NEAR account may self-claim one free Common/Basic NUVI;
- Dynamic NFT media:
  starter metadata derives `basic.png`, `form_a.png` or `form_b.png` from LivingState while preserving the same token ID;
- **NUVI NFT-controlled smart account**:
  - first mutation provisions a deterministic direct child account;
  - child executor is deployed with **no access keys**;
  - NFT metadata records the child address;
  - child is registered for NUVE and receives a Testnet-only 100 NUVE demo allocation;
  - every outgoing NUVE request fresh-checks the current owner of the bound NFT;
  - control follows NFT ownership without moving the child account or its assets;
- Hackathon web client:
  Testnet wallet connection derived from the production-tested NUVELOR near-connect path, bounded in-page Testnet gas sponsorship, direct NEAR RPC reads, starter claims, one-cycle gameplay, mutation, smart-account live proof, real NUVE transfer and explorer evidence.
- Judge Testnet sponsor:
  a fresh connected wallet may receive one bounded top-up to 2.0 NEAR before either Starter claim; the sponsor is Testnet-only and uses no Mainnet credential.

Current acceptance:

- Living NFT unit tests: **86/86 PASS**
- NUVI Account executor tests: **5/5 PASS**
- NUVE FT unit tests: **8/8 PASS**
- Web client tests: **6/6 PASS**
- Web production build: **PASS**
- Live Testnet smart-account E2E: **PASS**
- Public `nuvelor.gg` release: **LIVE**
- Fresh sponsor-funded judge cycle: **PASS** (deployments/judge-cycle-e2e-testnet.json)

## Live Testnet deployment

Hackathon accounts:

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- Sponsor account: sponsor-hack26.nvlborc70517034832.testnet
- E2E player: `player-hack26.nvlborc70517034832.testnet`
- NUVI `#1` account: `nuvi-1.nuvi-hack26.nvlborc70517034832.testnet`

Accepted WASM SHA-256:

- Living NFT: `721454ef99eac4d27bb043f761c835a9a40d79c5dda1a65593edab0e95d4ca1c`
- NUVI Account executor: `33ba2335814ecbbd94da66b4df5431e190cd3e2b70368a85deaea18081a8814c`

Live child-account proof for token `#1`:

- deterministic account exists;
- executor code is deployed;
- access key count: **0**;
- executor binding:
  - NFT contract: `nuvi-hack26.nvlborc70517034832.testnet`
  - token ID: `1`
  - FT contract: `nuve-hack26.nvlborc70517034832.testnet`
- token metadata contains the same `nuvi_account_id`;
- NUVE storage registration: active;
- Testnet demo allocation: `100.0000 NUVE`;
- two accepted proof transfers reduced the balance to `98.0000 NUVE`;
- non-owner calls were rejected;
- after NFT transfer the old owner was rejected and the new owner was authorized;
- the child address and balance were preserved across the ownership transfer itself;
- final NFT owner was restored to `player-hack26.nvlborc70517034832.testnet`.

Machine-readable evidence:

Current live / judge evidence:

- `deployments/current-testnet-state.json` — fresh finality RPC readback of the live Testnet code/config and zero-key NUVI Account state;
- `deployments/judge-cycle-e2e-testnet.json` — current accelerated fresh-wallet path: sponsor → Starter claims → one Work → mutation/provisioning → real NUVE transfer;
- `deployments/smart-account-e2e-testnet.json` — ownership-control invariant proof, including non-owner rejection and control transfer with NFT ownership.

Historical checkpoints retained for provenance (not the current runtime configuration):

- `deployments/testnet.json` — original Testnet deployment snapshot before the final one-Work acceleration;
- `deployments/e2e-testnet.json` — original five-Work Living NFT E2E;
- `deployments/smart-account-upgrade-testnet.json` — earlier smart-account upgrade checkpoint;
- `deployments/fresh-user-budget-2near-testnet.json` — earlier five-Work gas-budget measurement.

## Earlier Living NFT E2E

The original five-Work gameplay path remains accepted as historical evidence. The current Hackathon demo is intentionally accelerated to one Work cycle:

`starter NUVI → starter 100 NUVE → one Work A/B → XP 1 → mutation → keyless NUVI Account → 1 NUVE transfer`

Observed token `#1` gameplay state:

- same NEP-171 token ID: `1`
- final form: `form_b`
- XP: `5`
- Work A / Work B affinity: `3 / 2`
- Sponsor / Burner spend: `35 / 20 NUVE`
- mutation preview before execution: `57.14% Form A / 42.86% Form B`
- Global Alignment: `35 NUVE Treasury / 20 NUVE Burn`

The bounded Global Alignment bonus that will influence mutation is intentionally still pending; the repository does not claim it as a completed mechanic.

## Security boundary

Hackathon v0.1 intentionally proves a narrow authorization primitive.

The NUVI Account executor exposes the configured NUVE transfer path only. It does **not** expose:

- AddKey / FullAccess key creation;
- FunctionCall key creation;
- DeleteKey;
- arbitrary executor replacement;
- DeleteAccount;
- a general-purpose wallet surface.

Every supported outgoing action re-derives authority from current ownership of the bound NFT.

Known v0.1 limitation: NFT ownership is checked asynchronously before the downstream FT transfer. A production-grade generalized account must harden the owner-read/action race with controller-side pending-operation and NFT transfer/burn locking.

## Repository structure

- `contracts/living-nft` — isolated NEP-171 Living NFT
- `contracts/nuvi-account` — keyless NFT-controlled child-account executor
- `contracts/nuve-ft` — isolated NEP-141 demo NUVE
- `apps/web` — Hackathon Testnet demo UI
- `services/testnet-sponsor` — optional bounded guest onboarding
- `packages/shared` — shared public types/helpers
- `assets/forms` — generated public form images
- `deployments` — public Testnet deployment/E2E evidence
- `docs` — architecture, provenance, security and economy
- `scripts` — reproducible build/deploy/test helpers

Technical specification:

- `docs/NUVI_NFT_CONTROLLED_SMART_ACCOUNT_HACKATHON_DEMO_V01_TZ_2026-11-14.md`

Additional background:

- `docs/HACKATHON_BASELINE.md`
- `docs/DYNAMIC_MEDIA_V1.md`
- `docs/WEB_DEMO_V1.md`
- `docs/TESTNET_E2E_V1.md`

The private NUVELOR production repository remains separate. This public repository must never contain production secrets, private infrastructure configuration, AI_MEMORY data, deploy credentials, or private keys.
