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

Hackathon implementation is active on top of the disclosed pre-Hackathon baseline.

Implemented and source-tested:

- LivingState V4 foundation for persistent player-shaped NFT identity;
- isolated NUVE demo FT with one-time starter claim and bounded idempotent game rewards;
- Living Settlement V1 / contract state V5:
  `ft_transfer_call → Sponsor/Burn route → callback → Feed/Work timer`;
- Living Actions V1:
  ready Work A/B actions award XP and affinity; ready Feed actions mint a retry-safe 1 NUVE reward before the timer is cleared;
- Mutation V1:
  at 5 XP the current owner gets an on-chain Form A/Form B probability preview and may perform the single MVP mutation without changing `token_id`;
- Global Alignment Base / contract state V6:
  successful Treasury/Burn routes accumulate global NUVE totals and canonical basis-point shares.

The bounded Global Alignment bonus that will influence mutation is intentionally still pending; the repository does not claim the Memecoins With Mechanics track yet.

The private NUVELOR production repository remains separate. This repository must never contain production secrets, private infrastructure configuration, AI_MEMORY data, deploy credentials, or private keys.

See the implementation notes under `docs/`, including `GLOBAL_ALIGNMENT_V1.md`.

## Planned repository structure

- `contracts/living-nft` — isolated NEP-171 Living NFT
- `contracts/nuve-ft` — isolated NEP-141 demo NUVE
- `apps/web` — Hackathon demo UI
- `services/testnet-sponsor` — bounded Testnet guest onboarding
- `packages/shared` — shared public types/helpers
- `assets/forms` — public demo media references
- `docs` — architecture, provenance, security and economy
- `scripts` — reproducible build/deploy/test helpers

See `docs/HACKATHON_BASELINE.md`.
