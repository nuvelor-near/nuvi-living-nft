# NUVI Living Actions v1

Status: Hackathon-created implementation.

This layer completes the first playable Feed / Work loop after a paid action has passed Living Settlement V1.

## Completion rules

Only the current NFT owner may call `complete_living_action(token_id)`.

The action must exist and the NEAR block timestamp must be greater than or equal to `ready_at_ns`.

### Work A

On successful completion:

- `xp += 1`;
- `work_a_xp += 1`;
- `active_action = none`;
- `ready_at_ns = 0`.

### Work B

On successful completion:

- `xp += 1`;
- `work_b_xp += 1`;
- `active_action = none`;
- `ready_at_ns = 0`.

### Feed

A ready Feed does not clear immediately.

The Living NFT asks the isolated NUVE FT to mint:

`1 NUVE = 10,000 atomic units`

to the current NFT owner.

The reward ID is deterministic:

`feed:<token_id>:<ready_at_ns>`

The NUVE FT already persists processed reward IDs, so retries are idempotent.

The Feed timer is cleared only by a successful reward callback. If the reward call fails, the action remains ready and can be retried.

## NFT lock during actions

An NFT with an active Living action cannot be transferred or burned.

This prevents ownership from changing while an asynchronous Feed reward is between the Living NFT and NUVE FT contracts.

The lock ends when the action completes.

## Economy alignment

This implementation follows `docs/ECONOMY.md`:

- Feed reward: 1 NUVE;
- Work reward: 1 XP.

The prior NUVE FT documentation line that called 1 NUVE a Work reward was corrected because it conflicted with the existing economy table.

## Acceptance

Oracle source acceptance:

- Living NFT unit tests: `69/69 PASS`;
- NUVE FT unit tests: `8/8 PASS`;
- pinned Rust `1.86.0` Living NFT WASM build: PASS;
- Living NFT WASM SHA-256: `98ffd515a9b0d06f8328dc124a18e581b6150e8a40347ff5f3f2651c592dfb23`;
- Living NFT WASM size: `620348` bytes.

Next implementation gate: mutation threshold and form evolution.
