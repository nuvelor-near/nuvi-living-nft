# Global Alignment Base v1

Status: Hackathon-created implementation.

This commit establishes the aggregate NUVE signal needed for the planned community-driven Global Alignment mechanic without inventing the still-unspecified bounded bonus.

## State

Living NFT contract state V6 adds:

- `global_treasury_total`;
- `global_burn_total`.

Both values are raw atomic NUVE units.

## Counting rule

A value is counted only after the corresponding Living Settlement route succeeds.

- Sponsor / Treasury route → `global_treasury_total += amount`
- Burn route → `global_burn_total += amount`

A failed route is refunded through the existing NEP-141 flow and contributes zero to Global Alignment.

## View

`get_global_alignment()` returns:

- `treasury_total`;
- `burn_total`;
- `global_total`;
- `treasury_share_bps`;
- `burn_share_bps`.

Formula:

`global_total = treasury_total + burn_total`

When `global_total > 0`, Treasury and Burn shares sum to 10,000 basis points.

When no global activity exists yet, both shares are zero.

## Migration

`migrate_v5_to_v6()` preserves NFT, TokenClass, LivingState, configured NUVE/Treasury accounts and pending-settlement storage, and initializes the aggregate totals to zero.

Historical global route totals did not exist in V5, so they are not fabricated during migration.

## Track boundary

This base is necessary but not sufficient to claim **Memecoins With Mechanics**.

The previously planned next layer is a **bounded** Global Alignment influence on mutation probabilities. The exact maximum bonus was not fixed in the agreed MVP, so this implementation deliberately does not choose one unilaterally.

## Acceptance

Oracle source acceptance:

- Living NFT unit tests: `75/75 PASS`;
- NUVE FT unit tests: `8/8 PASS`;
- pinned Rust `1.86.0` Living NFT WASM build: PASS;
- Living NFT WASM SHA-256: `01836ca1678f2eb46d60963986726b88c6bcf42c98af96b6567c40e3f5ea12d5`;
- Living NFT WASM size: `647820` bytes.
