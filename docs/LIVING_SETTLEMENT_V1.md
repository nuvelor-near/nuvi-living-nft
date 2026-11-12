# NUVI Living Settlement v1

Status: Hackathon-created implementation.

This layer connects the isolated demo NUVE FT to Living NFT timed actions without touching production NUVELOR contracts.

## Contract state

The Living NFT root state advances from V4 to V5.

V5 adds:

- configured `nuve_ft_id`;
- configured `treasury_id`;
- one pending NUVE settlement per `token_id`;
- monotonic `settlement_id`.

`migrate_v4_to_v5(nuve_ft_id, treasury_id)` preserves the existing NFT identity, TokenClass data and LivingState. Direct V2/V3 to V5 migration paths are also provided for disclosed legacy fixtures.

## Payment message

Actions are initiated by NEP-141 `ft_transfer_call` to the Living NFT contract.

The `msg` field is JSON:

```json
{
  "token_id": "1",
  "action": "feed",
  "route": "sponsor"
}
```

Supported actions: `feed`, `work_a`, `work_b`.

Supported economic routes: `sponsor`, `burn`.

Only the configured NUVE FT contract is accepted. Only the current NFT owner may initiate a paid action for that NFT.

## Demo economy enforced on-chain

- Feed: `5 NUVE`, `30 s`
- Work A: `10 NUVE`, `10 s`
- Work B: `10 NUVE`, `10 s`

NUVE has 4 decimals, so the contract validates exact atomic amounts.

## Settlement sequence

1. NUVE FT transfers the requested amount to the Living NFT contract.
2. `ft_on_transfer` validates FT identity, NFT ownership, action state, amount and the absence of another pending settlement.
3. The Living NFT stores a pending settlement but does not mutate LivingState.
4. Sponsor route calls standard `ft_transfer` from the Living NFT FT balance to the configured Treasury account.
5. Burn route calls the demo NUVE contract's `burn` method from the Living NFT FT balance.
6. The callback finalizes only after the route promise succeeds.
7. On success, the appropriate economic counter is updated and the action timer starts.
8. On route failure, the callback returns the full transferred amount as unused so NEP-141 resolution can refund the payer.

## Failure and concurrency invariants

- failed routing does not start gameplay;
- failed routing does not increment Sponsor/Burn counters;
- only one payment may be pending per NFT;
- an NFT cannot be burned while its payment settlement is pending;
- transfer still preserves the NFT's LivingState identity;
- timer start happens only after successful economic routing;
- callbacks use settlement IDs so stale/mismatched callbacks cannot finalize a different action.

## Acceptance

Oracle source acceptance:

- Living NFT unit tests: `63/63 PASS`;
- pinned Rust `1.86.0` WASM build: PASS;
- WASM SHA-256: `82a5960b96bc88e662687b1531b87a71cd0cc8f2f39085398c7c9781de9dcfe3`;
- WASM size: `602559` bytes.

This commit does not yet implement completion rewards, XP settlement, mutation, Global Alignment, dynamic media, or Testnet deployment.
