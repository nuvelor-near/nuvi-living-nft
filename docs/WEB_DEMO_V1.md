# NUVI Hackathon Web Demo v1

Status: **LIVE on `nuvelor.gg`; Testnet smart-account proof accepted**.

## Public demo

Primary judge-facing route:

- `https://nuvelor.gg/en/near-hackathon-2026/nuvi-living-nft/`

Localized routes:

- `/uk/near-hackathon-2026/nuvi-living-nft/`
- `/de/near-hackathon-2026/nuvi-living-nft/`
- `/ru/near-hackathon-2026/nuvi-living-nft/`

The production public page embeds the Hackathon Testnet client directly. It does not require GitHub Pages.

## Wallet path

The Hackathon client intentionally reuses the proven NUVELOR connection architecture rather than creating a new wallet stack.

Source characteristics carried over:

- `@hot-labs/near-connect 0.11.1`;
- connector singleton;
- wallet session restoration;
- HOT manifest with fallback source;
- local HOT executor override;
- `wallet.signAndSendTransaction`.

Hackathon isolation changes:

- `network: testnet`;
- no Telegram;
- no referral binding;
- no NUVELOR production backend;
- no production contract IDs;
- no production credentials.

The copied HOT executor is byte-identical to the accepted source used for this implementation:

`25c62609d3cd2cee39f03eea20b3f6344353601030be0fb832472d534e34a2d4`

## Live contract defaults

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- Verified NUVI #1 account: `nuvi-1.nuvi-hack26.nvlborc70517034832.testnet`
- RPC: `https://rpc.testnet.fastnear.com`

The web app reads live Hackathon state directly and requires no private environment data.

## Demo flow implemented in UI

1. Connect NEAR Testnet wallet.
2. If the balance is below 2.0 NEAR, press **Get demo gas**; the bounded Testnet sponsor tops the fresh account up to 2.0 NEAR without leaving the page.
3. Claim one Starter NUVI.
4. Register FT storage when needed and claim 100 Starter NUVE.
5. Display owned NUVI, current form and LivingState.
6. Choose Treasury/Sponsor or Burn/Burner route.
7. Run one Work A or Work B through NUVE ft_transfer_call; Work settles in the same signed flow and immediately awards 1 XP.
8. Display XP, affinities, economic behavior and the on-chain Form A/Form B mutation probabilities.
9. Execute the one-time mutation.
10. Mutation starts deterministic keyless NUVI Account provisioning.
11. Display account status Not created / Provisioning / Active / Failed.
12. Once active, prove from live RPC that the child exists, has deployed code, has 0 access keys, matches the NFT/token/FT binding, is storage-registered and exposes live NUVE balance.
13. Execute a real 1.0000 NUVE transfer from the NUVI Account to the registered Testnet wallet.
14. Surface the resulting Testnet transaction as explorer evidence.

The page also contains a static judge-facing **Verified Testnet E2E** summary that remains useful without connecting a wallet.

## Smart-account proof

Token `#1` has a deterministic direct child:

`nuvi-1.nuvi-hack26.nvlborc70517034832.testnet`

Accepted live facts:

- Living NFT WASM SHA-256:
  `721454ef99eac4d27bb043f761c835a9a40d79c5dda1a65593edab0e95d4ca1c`
- child executor WASM SHA-256:
  `33ba2335814ecbbd94da66b4df5431e190cd3e2b70368a85deaea18081a8814c`
- access keys: `0`
- metadata status: `active`
- initial demo balance: `100.0000 NUVE`
- accepted proof balance after two owner-authorized transfers: `98.0000 NUVE`
- non-owner request before NFT transfer: rejected
- NFT transferred to second Testnet owner
- old owner request after transfer: rejected
- new NFT owner request: accepted
- child account/address/balance did not move with the NFT
- NFT returned to the original E2E owner after the proof

Machine-readable proof:

Current:
- `deployments/current-testnet-state.json`
- `deployments/judge-cycle-e2e-testnet.json`
- `deployments/smart-account-e2e-testnet.json`

Historical provenance:
- `deployments/smart-account-upgrade-testnet.json`
- `deployments/fresh-user-budget-2near-testnet.json`

## Public configuration

Public override values:

- `VITE_NFT_CONTRACT_ID`
- `VITE_NUVE_CONTRACT_ID`
- `VITE_TREASURY_ID`
- `VITE_DEMO_PLAYER_ID`
- `VITE_DEMO_MUTATION_TX`
- VITE_SPONSOR_URL (defaults to same-origin /api/hackathon/nuvi/testnet-fund)
- optional RPC/explorer overrides

No private key or deploy credential belongs in the web environment.

## Current acceptance

- Living NFT unit tests: `86/86 PASS`
- NUVI Account executor tests: `5/5 PASS`
- NUVE FT unit tests: `8/8 PASS`
- web client tests: `6/6 PASS`
- Vite production build: `PASS`
- embedded executor artifact parity: `PASS`
- live Living NFT code readback: `PASS`
- live child code readback: `PASS`
- live child zero-access-key proof: `PASS`
- live owner/non-owner authorization E2E: `PASS`
- live NFT ownership-transfer control inheritance E2E: `PASS`
- final NFT owner restoration: `PASS`
- public `nuvelor.gg` release: `LIVE`

GitHub Pages is not an acceptance dependency. The previous Pages activation blocker is superseded by the live `nuvelor.gg` route.

## Scope boundary

The Hackathon page deliberately proves the authorization foundation rather than becoming a general wallet.

Out of scope for v0.1 UI:

- generic NEAR send;
- arbitrary contract FunctionCall;
- NFT inventory/send UI;
- staking/DEX UI;
- key management;
- executor upgrades;
- DeleteAccount.

The product architecture may generalize the executor later, but access-key mutation remains outside the NFT-owner action surface.
