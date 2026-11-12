# NUVE Demo FT v1

Status: Hackathon-created implementation.

This contract is isolated from production `nuve.nuvelor.near`.

## Standards

- NEP-141 Fungible Token
- NEP-145 Storage Management
- NEP-148 Fungible Token Metadata

## Token

- name: `NUVI Demo Token`
- symbol: `NUVE`
- decimals: `4`
- initial supply: `0`

## Starter claim

Each registered Testnet account may claim once:

`100 NUVE = 1,000,000 atomic units`

The starter allocation is minted only when the account explicitly calls `starter_claim()`.

The account must already be registered under NEP-145.

## Gameplay reward adapter

Only the configured Living NFT contract may call `mint_game_reward()`.

Each reward carries a unique `reward_id`.

The contract stores processed reward IDs so a retried callback cannot mint the same reward twice.

The per-call guard is currently:

`100 NUVE`

The ordinary FT gameplay reward currently used by Living Actions V1 is the **Feed reward of 1 NUVE**. Work A/B award on-NFT XP instead. Therefore the 100 NUVE cap is a safety guard, not the normal emission amount.

## Burn

Any account may destroy its own NUVE with `burn()`.

Living Settlement V1 uses this for the Burn route:

1. player transfers NUVE to the Living NFT contract with `ft_transfer_call`;
2. the Living NFT validates and records a pending settlement;
3. the Living NFT burns NUVE held by its own FT account;
4. only the successful callback starts the requested Living action.

## Boundary

This contract does not import or modify:

- production `nuve.nuvelor.near`;
- production Claim;
- production Treasury;
- mainnet state;
- private NUVELOR infrastructure.

Feed/Work settlement, Treasury/Burn routing and the Feed reward callback are implemented only against this isolated Hackathon demo token.
