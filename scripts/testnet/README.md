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

## NFT-controlled NUVI Account upgrade

The smart-account demo upgrades only the existing isolated Testnet Living NFT contract. Dry-run is the default and fails closed unless the live predecessor SHA, V7 state/config, embedded executor digest and current NUVE storage minimum all match expectations.

~~~
node upgrade-smart-account-demo.mjs
~~~

If the dry-run reports a required Living storage top-up, execute with an explicitly selected project Testnet funding identity:

~~~
HACKATHON_FUNDING_ACCOUNT=<project-testnet-account> EXECUTE=1 \
  node upgrade-smart-account-demo.mjs
~~~

The upgrade writes public evidence to deployments/smart-account-upgrade-testnet.json.

## NFT-controlled NUVI Account E2E

Dry-run:

~~~
node smart-account-e2e.mjs
~~~

Execute (a funding identity is needed only if the existing demo player lacks the exact provisioning deposit plus gas reserve):

~~~
HACKATHON_FUNDING_ACCOUNT=<project-testnet-account> EXECUTE=1 \
  node smart-account-e2e.mjs
~~~

The E2E verifies a real keyless child account, live executor binding, NUVE storage and balance, authorized owner transfer, non-owner rejection, NFT ownership transfer -> controller transfer, unchanged child account/balance across the NFT transfer itself, and restoration of the original demo NFT owner. Evidence is written to deployments/smart-account-e2e-testnet.json.
