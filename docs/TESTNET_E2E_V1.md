> **Historical checkpoint.** This document records the original five-Work Testnet E2E. The current judge path is accelerated to one Work action; use `README.md`, `docs/WEB_DEMO_V1.md`, `deployments/current-testnet-state.json`, and `deployments/judge-cycle-e2e-testnet.json` for the current Hackathon runtime.

# NUVI Testnet E2E v1

Status: **LIVE PASS** on NEAR Testnet.

## Isolated deployment

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- E2E player: `player-hack26.nvlborc70517034832.testnet`

No production/Mainnet NUVELOR contract was changed.

## Deployed code identity

Living NFT:

- SHA-256: `64d4a735a029ffcc75bcd70c5498bc317426df60d4b2f57afbf86d76871b3813`
- size: `683959` bytes

NUVE FT:

- SHA-256: `c176a91f96007958564470f9a6e783fe6e3ef5ffbfe1b040dda1188db90c9f0e`
- size: `217437` bytes

Both code blobs were read back from Testnet and matched the locally accepted binaries.

## Executed demo loop

The live E2E used NFT token `#1`:

1. register player NUVE storage;
2. claim Starter NUVI;
3. claim 100 NUVE;
4. Feed through Treasury/Sponsor;
5. complete Feed and receive the 1 NUVE gameplay reward;
6. Work A through Sponsor;
7. Work A through Sponsor;
8. Work B through Burn;
9. Work B through Burn;
10. Work A through Sponsor;
11. reach XP 5;
12. read on-chain mutation probabilities;
13. mutate the same token ID.

The action mix deliberately exercises both economic routes and both work affinities.

## Final chain state

- token ID: `1`
- owner: `player-hack26.nvlborc70517034832.testnet`
- final form: `form_b`
- level: `1`
- XP: `5`
- Work A XP: `3`
- Work B XP: `2`
- Sponsor spent: `35 NUVE`
- Burner spent: `20 NUVE`
- mutation count: `1`
- active action: none

Mutation preview before execution:

- Form A weight: `80`
- Form B weight: `60`
- Form A probability: `57.14%`
- Form B probability: `42.86%`

Global Alignment after the loop:

- Treasury: `35 NUVE`
- Burn: `20 NUVE`
- total routed: `55 NUVE`
- Treasury share: `63.63%`
- Burn share: `36.37%`

NUVE balances after E2E:

- player: `46 NUVE`
- Treasury: `35 NUVE`
- total supply: `81 NUVE`

The supply arithmetic proves the two Work/Burn actions destroyed `20 NUVE`.

## Selected transaction evidence

- Player FT registration: `7VPTweHYcVLTs7zXM2EfPorbcQWcYgbuiSMufENGizWK`
- Starter NUVI: `7REoX7JwT5rvgnKxYKfj1fs7xU2FnVJTgxfEC8imLk3z`
- Starter NUVE: `35jQYkoAqJKNTxufttkXAMx64aUusnWdocFyetLAKqeE`
- Feed/Sponsor start: `8Pf2wLKMPr7FvkV4v1GSoGCVcgBbu158Phb6ey1T5619`
- Feed completion/reward: `2Txp4EB5k6rVT2DHzbBiJjz1SPNkLcFpsbK4szM7ruZM`
- first Burn-routed Work: `FE5KBJ173wKm7bP5bDiHuJ9QK9sw7sRpFqAWrfJb7zzE`
- final Work completion to XP 5: `3kw3DTWzeBzoKRrYPbE2qJiST5Q4TKZ4qyNhFyZH1n2B`
- mutation: `GdZRFghKgMzBoRV3W19TsDbkSafzKa59fUNTi39W1bYF`

The first four bootstrap hashes were independently re-queried after the resumable run and verified successful.

## Evidence files

- `deployments/testnet.json`
- `deployments/e2e-testnet.json`

These files contain no private keys or credentials.
