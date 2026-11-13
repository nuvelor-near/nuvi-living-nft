# NUVELOR — NUVI NFT-Controlled Smart Account
## Технічне завдання — Hackathon Demo v0.1

**Статус:** SCOPE FROZEN / READY FOR IMPLEMENTATION  
**Мережа:** NEAR Testnet only  
**Mainnet:** не змінювати в межах цього завдання  
**Baseline source:** `6f6a33be9c2642fb9c6f094a5b8a38279317e696`  
**Baseline acceptance:** Living NFT 81/81 PASS; NUVE FT 8/8 PASS; Web 4/4 PASS; build PASS.

## 1. Мета

Реалізувати мінімальний реальний on-chain proof: **NUVI NFT є переносним правом керування окремим keyless NEAR smart-account**.

Демо має довести:
1. після першої успішної мутації NUVI створюється окремий NEAR account;
2. account має deployed executor contract та 0 access keys;
3. account жорстко прив'язаний до конкретного NUVI NFT;
4. account реально володіє NUVE;
5. тільки поточний власник bound NFT може наказати smart-account відправити NUVE.

Повний account architecture надалі може охоплювати NEAR, FT, NFT, arbitrary FunctionCall, staking та інші звичайні on-chain дії, але Hackathon Demo v0.1 не повинен перетворюватися на general-purpose wallet UI.

## 2. Авторизація

Для кожного NUVI:

```text
NUVI NFT #N
   │ ownership
   ▼
current NFT owner
   │ request
   ▼
NUVI Smart Account
   │ fresh nft_token(N) owner check
   ▼
authorized / rejected
```

Приватного user FullAccess key для NUVI Smart Account немає. Єдине user-level джерело права керування — current owner bound NUVI NFT.

Після NFT transfer адреса smart-account, executor та його активи не змінюються; змінюється лише account, який проходить authorization check.

## 3. Адреса account

Формат Hackathon v0.1:

```text
nuvi-<numeric-token-id>.<living-nft-contract>
```

Приклад:

```text
token_id: 1
Living NFT: nuvi-hack26.nvlborc70517034832.testnet
NUVI Account: nuvi-1.nuvi-hack26.nvlborc70517034832.testnet
```

Використовувати direct deterministic subaccount Living NFT contract. Random/implicit адреси не використовувати.

## 4. Provisioning при мутації

Існуючий `mutate(token_id)` лишається user action.

Після успішного mutation state transition та `nuvi_mutated` event контракт запускає async provisioning:

```text
CreateAccount
→ Transfer initial NEAR
→ DeployContract executor WASM
→ FunctionCall new(nft_contract_id, token_id, nuve_ft_id)
```

Не виконувати `AddKey`. Account має створюватися keyless.

Mutation state не відкочується автоматично, якщо downstream provisioning promise згодом падає.

## 5. Executor contract

Новий crate:

```text
contracts/nuvi-account/
```

Immutable state:
- `nft_contract_id`
- `token_id`
- `nuve_ft_id`

Мінімальний ABI:
- `new(nft_contract_id, token_id, nuve_ft_id)`
- `get_config()`
- `request_ft_transfer(receiver_id, amount)`
- private callback `on_owner_checked(...)`

`request_ft_transfer`:
1. capture `env::predecessor_account_id()` as requester;
2. call bound Living NFT `nft_token(token_id)`;
3. callback parses current `owner_id`;
4. require `owner_id == requester`;
5. call configured NUVE `ft_transfer(receiver_id, amount)` with 1 yoctoNEAR;
6. FT sender is NUVI Smart Account itself.

Caller cannot override NFT contract, token id or FT contract.

## 6. Заборонені capabilities

Hackathon executor не повинен експонувати:
- AddKey;
- DeleteKey;
- FullAccess key creation;
- FunctionCall/gas-key creation;
- arbitrary replacement of executor WASM;
- DeleteAccount.

Це authorization boundary, а не звичайні user actions.

## 7. NUVE storage

Після executor init child account має пройти NEP-145 registration на NUVE.

Перед Testnet deployment точний deposit отримати через `storage_balance_bounds()`; не hardcode без fresh preflight.

Account не вважається `Active`, доки `storage_balance_of(child) != null`.

## 8. Demo allocation

Після успішного provisioning та storage registration дозволяється Testnet-only одноразово видати child account `100.0000 NUVE` через існуючий Living-authorized `mint_game_reward`.

Вимоги:
- receiver = child account;
- one-time/idempotent reward id, наприклад `nuvi-account:<token_id>:mutation:1`;
- `mutation_count == 1`;
- receiver storage-registered;
- amount не перевищує чинний 100 NUVE max.

Це demo allocation, не Mainnet economic decision.

## 9. NFT metadata

Після успішного core provisioning записати у stored `TokenMetadata.extra`:

```json
{"nuvi_account_id":"nuvi-1.nuvi-hack26.nvlborc70517034832.testnet"}
```

`nft_token(token_id)` повинен віддавати цю властивість on-chain. Не змінювати `LivingState` лише заради адреси; використовувати existing dynamic metadata merge.

## 10. Async UI state

Frontend states:
- Not created
- Provisioning
- Active
- Failed

Після mutation не показувати `Active` оптимістично. `Active` тільки після live verification account existence, code presence, 0 keys, metadata binding та NUVE storage registration.

## 11. Demo page — required evidence

У NUVI card додати компактний `NUVI ACCOUNT` block:
- connected wallet;
- NUVI token id;
- current form/image;
- XP / mutation result;
- current NFT owner;
- Account status;
- deterministic smart-account address + explorer link;
- Bound NFT;
- Controller = current NFT owner from live chain;
- Access keys = 0 / Locked from live RPC;
- Executor = deployed/code present from live RPC;
- NUVE balance;
- Quick Transfer: 1.0000 NUVE to connected wallet;
- optional custom Testnet recipient + NUVE amount;
- last outgoing transaction explorer link.

Live sources:
- `nft_token`
- `metadata.extra.nuvi_account_id`
- RPC `view_account`
- RPC `view_access_key_list`
- code hash
- NUVE `ft_balance_of`

## 12. Demo transfer

Primary judge action:

```text
Send 1 NUVE to my wallet
```

Receiver = connected wallet.

Preflight `storage_balance_of(receiver)`. Якщо receiver не зареєстрований, не відправляти і показати чітку помилку.

Після success:
- From = NUVI Smart Account;
- To = connected wallet;
- Amount = 1.0000 NUVE;
- Explorer transaction link;
- refresh smart-account balance.

Custom transfer — optional compact advanced form; не створювати general wallet UI.

## 13. Explicit non-scope UI

Не реалізовувати в Hackathon Demo v0.1:
- generic NEAR send;
- generic arbitrary FunctionCall;
- NFT inventory/send;
- staking;
- DEX interface;
- subaccount management;
- key management;
- executor upgrade;
- DeleteAccount;
- general-purpose wallet UI;
- dedicated NFT-transfer-control button.

## 14. Transfer-of-control acceptance proof

Окремий E2E test обов'язково:

```text
Alice owns NUVI #N
Alice sends NUVE from child → PASS

Alice transfers NUVI #N to Bob

Alice requests child transfer → REJECT
Bob requests child transfer → PASS
```

Після NFT transfer незмінні:
- child account id;
- executor;
- NUVE account;
- remaining NUVE balance.

Це головний proof inheritance of control.

## 15. Security boundary v0.1

Обов'язково:
- fresh owner check перед кожним outgoing request;
- immutable binding NFT contract/token id/NUVE contract;
- private callback;
- original requester captured before async query;
- amount > 0;
- no key-management methods;
- Mainnet untouched.

Відоме Hackathon limitation: async owner-read race між `nft_token()` і downstream `ft_transfer()`. Для production потрібні controller-side pending operation та NFT transfer/burn lock.

## 16. Build architecture

```text
build contracts/nuvi-account WASM
→ verify artifact
→ embed/include executor WASM into Living NFT build
→ build Living NFT
→ unit tests
→ Testnet deployment
```

Не завантажувати executor WASM runtime-ом із зовнішнього URL.

## 17. Regression gates

Обов'язково:
- existing Living NFT tests PASS;
- existing NUVE FT tests PASS;
- existing Web tests PASS;
- new executor tests PASS;
- new provisioning tests PASS;
- new frontend tests PASS;
- production web build PASS.

Accepted baseline: Living 81/81, NUVE 8/8, Web 4/4, build PASS.

## 18. Testnet acceptance

PASS тільки якщо live Testnet підтверджує:
1. Basic → Form A/B; mutation_count 0 → 1.
2. deterministic child exists.
3. child code hash is non-empty.
4. `view_access_key_list` returns 0 keys.
5. `nft_token().metadata.extra.nuvi_account_id == child account id`.
6. child is NUVE storage-registered and balance > 0.
7. current NFT owner can send 1 NUVE through executor.
8. non-owner cannot.
9. after NFT transfer old owner fails, new owner succeeds, child address and remaining balance unchanged.
10. regression gates remain PASS.
11. Testnet only; Mainnet untouched.

## 19. Definition of Done

Judge without CLI can see:
1. NUVI mutated.
2. NFT has `nuvi_account_id`.
3. real NEAR child account exists.
4. executor deployed.
5. account has 0 access keys.
6. current NFT owner shown as Controller.
7. account owns real NUVE.
8. current owner can send 1 NUVE from child.
9. confirmed Testnet transaction has Explorer link.

Separate E2E evidence proves NFT transfer → control transfer over the same smart-account and assets.

## 20. Scope freeze

До закриття v0.1 не розширювати scope на full wallet UI, arbitrary call UI, Mainnet, production recovery/governance/upgrades, multi-FT UX, generic NFT custody, staking, DEX чи external dApp account-abstraction.

Фундаментальний proof:

```text
NFT ownership
→ authorization
→ keyless NEAR smart account
→ real owned asset
→ real outgoing blockchain action
```

## 21. Implementation refinements verified on 2026-11-14

These refinements were introduced during implementation after fresh Testnet/runtime verification and do not expand the judge-facing scope.

### 21.1 Self-funded provisioning

`mutate(token_id)` is payable. Frontend reads the exact current value from `nuvi_account_creation_deposit()` immediately before signing.

The deposit covers:
- child executor contract storage stake;
- a bounded child operational NEAR reserve;
- exact live NUVE NEP-145 registration deposit.

Any excess attached deposit is returned to the caller.

Reason: the Living NFT contract must not depend on its own liquid balance to create every NUVI Account.

### 21.2 Existing mutated Testnet NUVI compatibility

The live accepted Testnet already contains mutated NUVI tokens created before account provisioning existed.

A bounded compatibility method `provision_nuvi_account(token_id)` is therefore allowed:
- current NFT owner only;
- token must already be mutated;
- only when no NUVI Account provisioning status exists;
- requires the same provisioning deposit;
- starts the same child-account provisioning pipeline.

It is not a replacement for automatic provisioning on new mutations. New mutations still start provisioning automatically.

### 21.3 On-chain provisioning status without V8 migration

Stored `TokenMetadata.extra` carries:
- `nuvi_account_status`: `provisioning | active | failed`;
- `nuvi_account_expected_id` while provisioning;
- `nuvi_account_id` only after successful activation;
- `nuvi_account_failure_phase` on failure.

This keeps the existing V7 contract state layout unchanged while making frontend status chain-derived.

### 21.4 Live Testnet storage fact

Fresh `nuve-hack26.nvlborc70517034832.testnet::storage_balance_bounds()` returned exactly `0.00125 NEAR` for registration.

Deployment code must still read/verify this value fresh before release instead of assuming it forever.

### 21.5 Global Contract disposition

NEAR Global Contracts were evaluated as a storage optimization. They reduce per-child code storage, but publishing this ~139 KB executor as a new global contract would currently burn roughly 13.9 Testnet NEAR under the protocol's global-contract storage pricing. That is disproportionate for this Hackathon demo and is not used in v0.1.

### 21.6 Build distinction

Oracle ARM64 can perform native Rust 1.86 development/test WASM builds. The canonical reproducible build image is AMD64 and cannot execute natively on Oracle without an emulation path. Release evidence must distinguish native Oracle build PASS from an AMD64 reproducible-build PASS.
