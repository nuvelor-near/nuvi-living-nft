# NUVI Hackathon Web Demo v1

Status: source implementation accepted; isolated Testnet deployment pending.

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

The copied HOT executor is byte-identical to the current NUVELOR source used for this implementation:

`25c62609d3cd2cee39f03eea20b3f6344353601030be0fb832472d534e34a2d4`

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

## Testnet configuration

The client uses public environment values only:

- `VITE_NFT_CONTRACT_ID`
- `VITE_NUVE_CONTRACT_ID`
- `VITE_TREASURY_ID`
- optional RPC/explorer overrides

No private key or deploy credential belongs in the web environment.

## Acceptance

- client unit tests: `4/4 PASS`
- Vite production build: PASS
- local preview HTTP smoke: PASS
- copied HOT executor HTTP/static SHA check: PASS
- NEAR Testnet RPC status: PASS
- connector manifest retrieval: PASS, HOT wallet present
- production account/backend string boundary audit: PASS
- secret-pattern audit: PASS

Current built JavaScript is approximately 217 KB before gzip and approximately 66 KB gzip.

## Next gate

Bind isolated Hackathon Testnet accounts, deploy both contracts, configure collection base URI, register NFT/Treasury FT storage, and execute the real wallet E2E flow.
