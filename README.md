# NUVI Living NFT

**NEAR Hackathon 2026 — public development repository**

NUVI is a player-shaped Living NFT: the same NEP-171 token keeps its identity while verified on-chain actions shape its experience, economic behavior, form, media, and mutation path.

> The player does not collect the character. The player creates who the character becomes.

## Hackathon tracks

Core:

- **NFTs That Do Something**
- **Games With Real Economies**
- **Open Innovation**

Conditional after implementation and acceptance of the Global NUVE mechanic:

- **Memecoins With Mechanics**

Stretch only after the Testnet core passes acceptance:

- Mainnet deployment
- NEAR Intents

## Current status

This commit establishes the public Hackathon repository boundary and documentation baseline.

**No Hackathon gameplay implementation is claimed complete at this baseline.**

The private NUVELOR production repository remains separate. This repository must never contain production secrets, private infrastructure configuration, AI_MEMORY data, deploy credentials, or private keys.

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
