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

## Living Settlement V1

Implemented:

- only configured demo NUVE FT may enter the payment receiver;
- exact action cost is enforced on-chain;
- only current NFT owner may initiate payment;
- one pending settlement per NFT;
- LivingState changes only after successful Sponsor/Burn routing;
- failed route returns the full amount for NEP-141 refund resolution;
- burn is blocked while a settlement is pending;
- callback settlement IDs prevent stale settlement finalization.

## Living Actions V1

Implemented:

- only the current NFT owner may complete an action;
- completion before `ready_at_ns` is rejected;
- Work A/B completion mutates XP synchronously;
- Feed keeps its action lock until the NUVE reward call succeeds;
- Feed reward IDs are deterministic from `token_id + ready_at_ns`, so a retry cannot mint twice;
- failed Feed reward calls leave the ready action retryable;
- transfer and burn are blocked while a Living action is active, preventing owner changes during asynchronous reward settlement.
