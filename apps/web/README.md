# NUVI Hackathon Web Demo

Minimal Testnet UI for the isolated NUVI Living NFT Hackathon contracts.

## Wallet connector

The connector is derived from the production-tested NUVELOR wallet path:

- `@hot-labs/near-connect 0.11.1`
- local HOT wallet executor override
- `wallet.signAndSendTransaction`

Hackathon differences are deliberate:

- network is Testnet;
- no Telegram binding;
- no referral flow;
- no NUVELOR production backend;
- no production contract IDs.

## Configure

Copy `.env.example` to `.env.local` and set the three isolated Hackathon Testnet account IDs after deployment.

## Forms

Place generated NFT images in repository-level `assets/forms/` using exact names:

- `basic.png`
- `form_a.png`
- `form_b.png`

`npm run dev` and `npm run build` copy them into the web public `/forms/` path.

The Living NFT collection `base_uri` should be set to the deployed demo URL plus `/forms`.

## Commands

```bash
npm install
npm test
npm run build
npm run dev
```
