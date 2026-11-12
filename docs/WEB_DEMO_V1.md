# NUVI Hackathon Web Demo v1

Status: **source + Testnet integration accepted; public Pages activation pending**.

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

The copied HOT executor is byte-identical to the NUVELOR source used for this implementation:

`25c62609d3cd2cee39f03eea20b3f6344353601030be0fb832472d534e34a2d4`

## Live contract defaults

- Living NFT: `nuvi-hack26.nvlborc70517034832.testnet`
- NUVE FT: `nuve-hack26.nvlborc70517034832.testnet`
- Treasury: `treasury-hack26.nvlborc70517034832.testnet`
- RPC: `https://rpc.testnet.fastnear.com`

The web app therefore reads the live Hackathon deployment without a local environment file.

## Demo flow implemented in UI

1. Connect NEAR wallet.
2. Claim one Starter NUVI.
3. Register FT storage when needed and claim 100 Starter NUVE.
4. Display owned NUVI, current form and LivingState.
5. Choose Treasury/Sponsor or Burn/Burner route.
6. Start Feed, Work A or Work B through NUVE `ft_transfer_call`.
7. Display active timer and complete the action when ready.
8. Display XP, affinities and economic behavior.
9. Display on-chain Form A/Form B mutation probabilities.
10. Execute the one-time mutation.
11. Display Global Alignment totals and shares.
12. Surface transaction hashes as Testnet explorer evidence.

The page also includes a **Live chain proof** panel that requires no wallet. It reads the already-verified E2E player/token state directly from Testnet and links to the player, contract and mutation transaction.

## Testnet configuration

Public override values:

- `VITE_NFT_CONTRACT_ID`
- `VITE_NUVE_CONTRACT_ID`
- `VITE_TREASURY_ID`
- `VITE_DEMO_PLAYER_ID`
- `VITE_DEMO_MUTATION_TX`
- optional RPC/explorer overrides

No private key or deploy credential belongs in the web environment.

## Acceptance

- client unit tests: `4/4 PASS`
- Vite production build: PASS
- GitHub Pages subpath build: PASS
- copied HOT executor SHA check: PASS
- live Testnet contract E2E: PASS
- production account/backend boundary audit: PASS

GitHub Actions run `43285807520`:

- build job: PASS;
- static Pages artifact: PASS;
- deploy job: blocked only because the repository Pages site is not yet enabled with Source = GitHub Actions.

## Next gate

1. add `basic.png`, `form_a.png`, `form_b.png`;
2. enable GitHub Pages for the repository using GitHub Actions;
3. rerun Pages deployment;
4. set collection `base_uri` with `scripts/testnet/set-media-base.mjs`;
5. run browser/wallet acceptance and capture submission evidence.
