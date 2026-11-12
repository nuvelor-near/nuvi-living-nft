# NUVI Living NFT

**NEAR Hackathon 2026 — public development repository**

NUVI is a player-shaped Living NFT: the same NEP-171 token keeps its identity while verified on-chain actions shape its experience, economic behavior, form, media, and mutation path.

> The player does not collect the character. The player creates who the character becomes.

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

Implemented and source-tested:

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
  starter metadata derives `basic.png`, `form_a.png` or `form_b.png` from the LivingState form and collection `base_uri` while preserving the same token ID;
- Hackathon web client:
  Testnet wallet connection derived from the production-tested NUVELOR `@hot-labs/near-connect` path, direct NEAR RPC reads, starter claims, gameplay, mutation, Global Alignment and explorer evidence.

Current acceptance:

- Living NFT unit tests: **81/81 PASS**
- NUVE FT unit tests: **8/8 PASS**
- Web client tests: **4/4 PASS**
- Web production build: **PASS**
- Web preview smoke: **PASS**
- Testnet RPC: **PASS**
- Production-boundary audit: **PASS**

The generated form images are intentionally separate from code. Add them under `assets/forms/` as `basic.png`, `form_a.png`, and `form_b.png`.

The bounded Global Alignment bonus that will influence mutation is intentionally still pending; the repository does not claim the Memecoins With Mechanics track yet.

The private NUVELOR production repository remains separate. This repository must never contain production secrets, private infrastructure configuration, AI_MEMORY data, deploy credentials, or private keys.

## Repository structure

- `contracts/living-nft` — isolated NEP-171 Living NFT
- `contracts/nuve-ft` — isolated NEP-141 demo NUVE
- `apps/web` — Hackathon Testnet demo UI
- `services/testnet-sponsor` — optional bounded guest onboarding
- `packages/shared` — shared public types/helpers
- `assets/forms` — generated public form images
- `docs` — architecture, provenance, security and economy
- `scripts` — reproducible build/deploy/test helpers

See `docs/HACKATHON_BASELINE.md`, `docs/DYNAMIC_MEDIA_V1.md` and `docs/WEB_DEMO_V1.md`.
