# NUVI Living NFT

**NEAR Hackathon 2026 — public development repository**

NUVI is a player-shaped Living NFT: the same NEP-171 token keeps its identity while verified on-chain actions shape its experience, economic behavior, form, media, mutation path, and control of a separate keyless NEAR account.

> The player does not collect the character. The player creates who the character becomes.

## Live demo

**Interactive Testnet demo:** https://nuvelor.gg/en/near-hackathon-2026/nuvi-living-nft/

Localized versions are also available under `/uk/`, `/de/`, and `/ru/`.

The demo is isolated from NUVELOR Mainnet/production contracts. Wallet-signed actions use NEAR Testnet only.

## 60-second judge path

The shortest proof does not require trusting the UI:

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
  `ft_transfer_call → Sponsor/Burn route → callback → Feed/Work timer`;
- Living Actions V1:
  Work A/B award XP and affinity; Feed mints a retry-safe 1 NUVE reward;
- Mutation V1:
  at 5 XP the current owner gets an on-chain Form A/Form B probability preview and may perform the single MVP mutation without changing `token_id`;
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
  Testnet wallet connection derived from the production-tested NUVELOR `@hot-labs/near-connect` path, direct NEAR RPC reads, starter claims, gameplay, mutation, smart-account live proof, real NUVE transfer and explorer evidence.

Current acceptance:

- Living NFT unit tests: **86/86 PASS**
- NUVI Account executor tests: **5/5 PASS**
- NUVE FT unit tests: **8/8 PASS**
- Web client tests: **6/6 PASS**
- Web production build: **PASS**
- Live Testnet smart-account E2E: **PASS**
- Public `nuvelor.gg` release: **LIVE**

## Live Testnet deployment

Hackathon accounts:

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- E2E player: `player-hack26.nvlborc70517034832.testnet`
- NUVI `#1` account: `nuvi-1.nuvi-hack26.nvlborc70517034832.testnet`

Accepted WASM SHA-256:

- Living NFT: `c76e557543d127b023f07d435211c19a8ed040967b7bb823ee27492952c91f91`
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

- `deployments/testnet.json`
- `deployments/e2e-testnet.json`
- `deployments/smart-account-upgrade-testnet.json`
- `deployments/smart-account-e2e-testnet.json`

## Earlier Living NFT E2E

The original gameplay path remains accepted:

`starter NUVI → starter 100 NUVE → Feed/Sponsor → Feed reward → 5 Work actions across Sponsor/Burn → XP 5 → mutation`

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
