# NUVI Hackathon Testnet tooling

These scripts deploy and exercise only the isolated public Hackathon contracts.

## Safety boundary

- Testnet only.
- Existing private keys are read from the normal NEAR credentials directory and are never copied into the repository or printed.
- New subaccount credentials are written only to `~/.near-credentials/testnet/` with mode 0600.
- A different pre-existing contract code hash fails closed.
- Deployment writes public evidence to `deployments/testnet.json`.

## Default isolated accounts

Parent:

`nvlborc70517034832.testnet`

Children:

- `nuvi-hack26.<parent>`
- `nuve-hack26.<parent>`
- `treasury-hack26.<parent>`
- `player-hack26.<parent>`

All IDs and allocations can be overridden with environment variables.

## Deploy

Dry-run is the default:

```bash
node deploy.mjs
```

Execute:

```bash
EXECUTE=1 node deploy.mjs
```

The script is designed to be safely re-run after a partial failure: existing accounts require their matching local credential, identical WASM is accepted, and conflicting code/configuration fails closed.

## Gameplay E2E

```bash
node e2e.mjs
```

The E2E runner is resumable. It reads chain state before each step and does not replay completed starter claims or a completed mutation. It writes public evidence to `deployments/e2e-testnet.json`.

## Dynamic media base URI

After the three form images are public, first dry-run:

```bash
PUBLIC_FORMS_BASE_URI=https://nuvelor-near.github.io/nuvi-living-nft/forms \
  node set-media-base.mjs
```

Then execute:

```bash
PUBLIC_FORMS_BASE_URI=https://nuvelor-near.github.io/nuvi-living-nft/forms \
  EXECUTE=1 node set-media-base.mjs
```

The script preserves all existing collection metadata, changes only `base_uri`, requires HTTPS, reads the new value back, and verifies that token `#1` resolves to `<base_uri>/<current_form>.png`.
