# Demo Economy

| Parameter | Default |
|---|---:|
| Starter NUVI | 1/account |
| Starter NUVE | 100 NUVE |
| Feed cost | 5 NUVE |
| Feed duration | 30 sec |
| Feed reward | 1 NUVE |
| Work cost | 10 NUVE |
| Work duration | 10 sec |
| Work reward | 1 XP |
| Mutation threshold | 5 XP |

Each paid action supports:

- **Treasury** — contributes to Sponsor behavior.
- **Burn** — reduces supply and contributes to Burner behavior.

## Mutation V1

The first MVP mutation is `basic → form_a/form_b`.

Behavior levels are derived from raw NUVE spent through each route:

| NUVE spent | level |
|---|---:|
| 0–4.9999 | 0 |
| 5–19.9999 | 1 |
| 20–39.9999 | 2 |
| ≥40 | 3 |

Weights are calculated on-chain:

- Form A = `10 + work_a_xp × 20 + sponsor_level × 5`
- Form B = `10 + work_b_xp × 20 + burner_level × 5`

`get_mutation_preview(token_id)` returns normalized basis-point probabilities. The frontend must display these values rather than recalculate them.

Mutation V1 is weighted random and single-use. It preserves `token_id`; XP is not consumed because repeated-mutation/XP-consumption rules are outside the agreed MVP.

Aggregate Treasury/Burn behavior is planned to form **Global Alignment**.
