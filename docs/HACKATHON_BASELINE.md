# Hackathon Baseline

## Existing before this repository

NUVELOR existed before NEAR Hackathon 2026 and already included private work such as:

- NEP-171 NFT implementation;
- TokenClass / NFT V3 work;
- NEP-141 NUVE token implementation;
- Claim concepts and implementations;
- broader NUVELOR economy and gameplay architecture.

Those components are not automatically claimed as Hackathon-created work.

## Public baseline

At tag:

`pre-hackathon-baseline`

this repository contains repository structure and public documentation only.

No completed Living NFT Hackathon gameplay implementation exists at this baseline.

## Reuse policy

Any later reuse of private NUVELOR code must record:

- source repository;
- source branch;
- exact source commit SHA;
- imported files;
- what existed before the Hackathon;
- what is newly developed here.

Never copy production credentials, private infrastructure configuration, AI_MEMORY, private keys, tokens, JWTs, or unrelated proprietary source.

## Imported pre-existing NFT V3 foundation

The following NFT foundation is explicitly disclosed as
**pre-existing NUVELOR work** and is not claimed as newly created
during NEAR Hackathon 2026.

Source:

- private repository: `urw13/NUVELOR`
- branch: `feature/nft-v3-tokenclass`
- exact source head:
  `7ba110d393333d361e1a8e8b5e8311e5f4971a56`
- implementation commit:
  `c995f4046542ea4f759543faed3f178b50d8884f`
- Testnet acceptance commit:
  `fdac3b2fd4296e110fe8f645365db26e0c3281c9`
- prior Testnet result: PASS
- prior NFT regression suite: 54/54 PASS

Imported files:

- `chain/nft/Cargo.toml`
- `chain/nft/Cargo.lock`
- `chain/nft/src/lib.rs`
- `chain/nft/src/types.rs`
- `chain/nft/src/events.rs`
- `chain/nft/src/tests/mod.rs`
- `chain/nft/src/tests/unit.rs`
- `chain/nft/src/tests/integration.rs`

Destination:

- `contracts/living-nft/`

The import intentionally excludes:

- mainnet deployment material;
- prior Testnet deployment artifacts;
- production configuration;
- private infrastructure;
- credentials and secrets;
- unrelated NUVELOR source.

Hackathon-created Living NFT functionality will be developed in
subsequent commits on top of this explicitly disclosed baseline.
