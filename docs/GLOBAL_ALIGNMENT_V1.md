# Global Alignment Base v1

Status: Hackathon-created implementation.

This layer establishes the aggregate NUVE signal needed for the planned community-driven Global Alignment mechanic without inventing the still-unspecified bounded bonus.

## State

The aggregate counters were introduced in contract state V6 and are preserved in current V7:

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

Current V7 provides direct legacy migration paths and `migrate_v6_to_v7()`.

V6 → V7 preserves both Global Alignment totals exactly. Older states that never stored global totals initialize them to zero rather than fabricating historical activity.

## Track boundary

This base is necessary but not sufficient to claim **Memecoins With Mechanics**.

The previously planned next layer is a **bounded** Global Alignment influence on mutation probabilities. The exact maximum bonus was not fixed in the agreed MVP, so this implementation deliberately does not choose one unilaterally.

## Acceptance

The Global Alignment base was accepted at 75/75 Living NFT tests before the subsequent Starter NUVI V7 extension. Current V7 acceptance is recorded in `STARTER_NUVI_V1.md`.
