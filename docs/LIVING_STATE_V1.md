# NUVI LivingState v1

Status: Hackathon-created implementation.

This is the first new Living NFT state layer implemented after the
disclosed `pre-hackathon-baseline` and NFT V3 import.

## Contract state version

The imported NFT V3 root layout is preserved as `ContractV3`.

The new contract layout is **V4** and appends:

`living_state_by_id: LookupMap<TokenId, LivingState>`

A dedicated `migrate_v3_to_v4()` migration initializes default
LivingState for every currently live NFT.

A direct `migrate_v2_to_v4()` path is retained for old V2 fixtures.

## LivingState

Each live NFT receives persistent state keyed by its existing `token_id`:

- `form_id`
- `level`
- `xp`
- `specialization`
- `mutation_count`
- `work_a_xp`
- `work_b_xp`
- `sponsor_spent`
- `burner_spent`
- `active_action`
- `ready_at_ns`

Initial values:

- form: `basic`
- level: `1`
- XP and counters: `0`
- specialization: none
- active action: none
- timer: `0`

`sponsor_spent` and `burner_spent` are raw atomic NUVE amounts.
Their JSON view uses NEAR `U128` string encoding.

## Identity invariant

LivingState belongs to the NFT identity, not to an account.

Therefore:

- `nft_transfer` does not reset LivingState;
- `nft_transfer_call` and resolver rollback do not reset LivingState;
- the new owner inherits the same character state;
- the previous owner loses NFT authority through the existing
  ownership rules;
- burn removes the live LivingState;
- mutation will later change `form_id` while preserving `token_id`.

## Not implemented in this commit

This state-foundation commit does **not** yet implement:

- Feed;
- Work A / Work B settlement;
- NUVE payment callbacks;
- Treasury/Burn routing;
- XP rewards;
- mutation weighting;
- Global Alignment;
- dynamic media mutation.

Those mechanics will be layered on top of this state model in
subsequent Hackathon commits.
