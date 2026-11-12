# Starter NUVI v1

Status: Hackathon-created implementation.

Starter NUVI makes the NFT side of the demo self-service.

## Rule

Each NEAR account may call:

`claim_starter_nuvi()`

exactly once.

The claim is free to the caller. For the Hackathon demo, the Living NFT contract subsidizes the storage consumed by this starter mint.

The persistent claim flag is account-based and is never cleared by transfer or burn.

## Initial NFT

The starter is minted directly to the caller with:

- base rarity: `Common`;
- form: `basic`;
- level: `1`;
- XP: `0`;
- mutation count: `0`.

The NEP-171 token receives a normal LivingState immediately.

Contract-defined metadata uses:

- title: `NUVI Starter`;
- description: `Nuvi is a mysterious creature powered by NUVELOR energy. It farms NUVE and evolves with the player.`;
- one copy;
- no fabricated external media URL.

Dynamic form media is a separate implementation gate.

## Security

- caller cannot supply arbitrary starter metadata;
- mint pause disables starter claims;
- second claim from the same account is rejected even after the original starter is transferred or burned;
- different NEAR accounts may each claim one starter;
- no production account, key or contract is involved.

## State migration

Contract state V7 adds the account-scoped starter-claim map.

`migrate_v6_to_v7()` preserves NFTs, LivingState, NUVE/Treasury configuration and Global Alignment totals, then initializes the starter-claim map empty.

No historical starter claim is fabricated because this public Hackathon starter method did not exist in V6.

## Acceptance

Oracle source acceptance:

- Living NFT unit tests: `80/80 PASS`;
- NUVE FT unit tests: `8/8 PASS`;
- pinned Rust `1.86.0` Living NFT WASM build: PASS;
- Living NFT WASM SHA-256: `626f6e3e76f6e8cb4ccc5323ac0487b9d94c9cf9ad1d3f6f301e48c9c13d56f4`;
- Living NFT WASM size: `673658` bytes.
