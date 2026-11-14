# NUVI Mutation v1

Status: Hackathon-created implementation.

Mutation V1 turns the player's accumulated Work and economic choices into the first visible Living NFT form change.

## Eligibility

The NFT is eligible when all conditions hold:

- current `form_id = basic`;
- `mutation_count = 0`;
- `xp >= WORK_CYCLES_PER_MUTATION`; the Hackathon demo sets `WORK_CYCLES_PER_MUTATION = 1`;
- no active Feed/Work action;
- no pending NUVE settlement.

The one-cycle threshold is a demo acceleration, not a frozen production economy rule. The production/gameplay cycle count will be reviewed against the concrete economy before release.

Only the current NFT owner may execute `mutate(token_id)`.

## Preview

`get_mutation_preview(token_id)` is the canonical probability source.

It returns:

- current form and mutation stage;
- XP and threshold;
- Work A / Work B XP;
- Sponsor / Burner behavior levels;
- Form A / Form B weights;
- normalized probabilities in basis points.

The web UI must not duplicate the weighting formula.

## Behavior levels

Raw `sponsor_spent` and `burner_spent` atomic NUVE values map to levels:

- below 5 NUVE → 0;
- 5 to below 20 NUVE → 1;
- 20 to below 40 NUVE → 2;
- 40 NUVE or more → 3.

## Weights

- Form A: `10 + work_a_xp × 20 + sponsor_level × 5`
- Form B: `10 + work_b_xp × 20 + burner_level × 5`

## Selection

Mutation is weighted random.

The contract combines NEAR random seed with:

- `token_id`;
- current `mutation_count`;
- block height;
- block timestamp.

The resulting SHA-256 prefix is reduced against the total weight and selects `form_a` or `form_b`.

This is a Hackathon gameplay mechanic, not a source of randomness for high-value financial settlement.

## State changes

Mutation V1:

- preserves the NEP-171 `token_id`;
- changes `form_id` from `basic` to `form_a` or `form_b`;
- increments `mutation_count` from 0 to 1;
- does not consume XP;
- does not change `level`;
- leaves `specialization` unset.

A second mutation is rejected in the MVP.

The contract emits `nuvi_mutated` with the selected form and the on-chain inputs/weights used for the choice.

## Acceptance

Oracle source acceptance:

- Living NFT unit tests: `73/73 PASS`;
- NUVE FT unit tests: `8/8 PASS`;
- pinned Rust `1.86.0` Living NFT WASM build: PASS;
- Living NFT WASM SHA-256: `f0de20412dbd5ccc6220202dbf54a661f373f2e3c45f4b0c8ec1934175ee52e8`;
- Living NFT WASM size: `633938` bytes.
