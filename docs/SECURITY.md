# Security Boundary

Forbidden in this repository:

- production private keys;
- seed phrases;
- API tokens;
- Pinata JWT;
- production `.env`;
- AI_MEMORY contents;
- NuViS credentials;
- HostPro/Oracle private configuration;
- private NUVELOR deploy credentials.

Planned invariants:

- only current NFT owner may act;
- starter NFT once/account;
- starter NUVE once/account;
- reward/work settlement idempotent;
- failed FT settlement does not mutate gameplay state;
- mutation preserves token_id;
- transfer preserves LivingState/history;
- previous owner loses authority immediately.
