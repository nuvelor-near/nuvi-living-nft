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
