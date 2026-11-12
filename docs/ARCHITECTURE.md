# Architecture

## Living NFT contract

- NEP-171 ownership — implemented
- one free self-service starter NUVI per account — implemented
- LivingState — implemented
- Feed / Work payment settlement and timers — implemented
- XP / action completion — implemented
- retry-safe Feed NUVE reward — implemented
- personal economic behavior counters — implemented
- mutation preview + first Form A/Form B mutation — implemented
- Global NUVE Treasury/Burn totals + bps view — implemented
- dynamic metadata/media from LivingState form + collection base URI — implemented
- bounded community alignment influence — pending exact agreed cap

## NUVE Demo FT

- NEP-141 — implemented
- one-time starter claim — implemented
- Treasury route — implemented
- Burn route — implemented
- bounded idempotent gameplay reward minting — implemented

## Web demo

Implemented:

- Testnet-only `@hot-labs/near-connect` wallet layer derived from NUVELOR production connector;
- exact local HOT wallet executor copied from NUVELOR and SHA-verified;
- direct NEAR JSON-RPC view calls;
- one-time Starter NUVI claim;
- FT storage registration + Starter NUVE claim;
- NUVE balance and NFT/LivingState reads;
- Feed / Work A / Work B with Treasury or Burn route;
- timer and completion UI;
- mutation probability preview and mutation call;
- Global Alignment totals/shares;
- Testnet explorer links and recent transaction evidence;
- responsive mobile layout;
- generated form asset sync into public `/forms/`.

## Testnet execution

Live isolated accounts:

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- E2E player: `player-hack26.nvlborc70517034832.testnet`

Verified:

- deployed WASM readback against local SHA-256 — PASS;
- FT storage prerequisites — PASS;
- starter NUVI + starter NUVE — PASS;
- Feed/Sponsor + asynchronous NUVE reward — PASS;
- Work A/B through both Sponsor and Burn routes — PASS;
- XP threshold and on-chain mutation preview — PASS;
- same-token mutation to Form B — PASS;
- Global Alignment and NUVE supply arithmetic — PASS.

Public machine evidence lives in `deployments/`.

Next critical gate:

- add final generated form images;
- publish the static web client;
- set collection `base_uri` to the public `/forms` directory;
- verify wallet/browser demo against the deployed Testnet contracts;
- capture screenshots/video and submission evidence.

## Testnet sponsor

Deferred until wallet-first Testnet E2E is proven.

- bounded Testnet funding only;
- no production credentials;
- add only if judge onboarding needs it.

Production NUVELOR contracts are outside this Hackathon execution boundary.
