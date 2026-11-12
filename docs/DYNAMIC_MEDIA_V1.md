# NUVI Dynamic Media v1

Status: Hackathon-created implementation.

The Living NFT keeps one NEP-171 token identity while its public media follows the current LivingState form.

## Convention

The collection metadata `base_uri` points at a public form directory.

Starter NUVI stores no explicit token-level media. Therefore the public `nft_token` view derives:

- `basic` → `<base_uri>/basic.png`
- `form_a` → `<base_uri>/form_a.png`
- `form_b` → `<base_uri>/form_b.png`

After mutation the same token ID immediately resolves to the new form image.

An explicitly configured token-level `media` remains authoritative and is not overwritten.

## Dynamic extra metadata

The public token view merges current LivingState fields into NEP-171 `metadata.extra`:

- `form_id`
- `level`
- `xp`
- `mutation_count`
- `work_a_xp`
- `work_b_xp`
- `sponsor_spent`
- `burner_spent`
- `active_action`
- `ready_at_ns`

Existing JSON object fields such as character/rarity are preserved.

## Artwork handoff

Generated artwork is stored under repository-level `assets/forms/` using exact filenames:

- `basic.png`
- `form_a.png`
- `form_b.png`

The web client's build step copies available images to its public `/forms/` directory.

No contract upgrade or state migration is required when the final image files arrive.

## Acceptance

- Living NFT unit tests: `81/81 PASS`
- dynamic Basic media test: PASS
- mutation changes derived media while preserving token ID: PASS
- explicit token media preservation: PASS
- pinned Rust 1.86.0 WASM build: PASS
- current WASM SHA-256: `64d4a735a029ffcc75bcd70c5498bc317426df60d4b2f57afbf86d76871b3813`
- current WASM size: `683959` bytes
