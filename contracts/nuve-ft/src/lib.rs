use borsh::BorshSerialize;
use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::collections::{LazyOption, LookupMap};
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, near, AccountId, BorshStorageKey, NearToken, PanicOnDefault,
    PromiseOrValue,
};

pub const NUVE_DECIMALS: u8 = 4;
pub const NUVE_UNIT: u128 = 10_000;
pub const STARTER_CLAIM_ATOMIC: u128 = 100 * NUVE_UNIT;

/// Safety bound for one game-reward callback.
/// It is deliberately much larger than the planned 1 NUVE gameplay reward,
/// while preventing an accidental single-call unbounded mint.
pub const MAX_GAME_REWARD_ATOMIC: u128 = 100 * NUVE_UNIT;

const MAX_REWARD_ID_BYTES: usize = 128;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Token,
    Metadata,
    StarterClaimed,
    RewardIds,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    reward_minter_id: AccountId,

    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,

    starter_claimed: LookupMap<AccountId, bool>,
    processed_reward_ids: LookupMap<String, bool>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, reward_minter_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Contract is already initialized");

        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "NUVI Demo Token".to_string(),
            symbol: "NUVE".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: NUVE_DECIMALS,
        };
        metadata.assert_valid();

        let mut token = FungibleToken::new(StorageKey::Token);

        // Owner is registered so contract administration/tests never need
        // an artificial mint solely to establish FT storage.
        token.internal_register_account(&owner_id);

        Self {
            owner_id,
            reward_minter_id,
            token,
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            starter_claimed: LookupMap::new(StorageKey::StarterClaimed),
            processed_reward_ids: LookupMap::new(StorageKey::RewardIds),
        }
    }

    pub fn owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn reward_minter_id(&self) -> AccountId {
        self.reward_minter_id.clone()
    }

    pub fn starter_claim_amount(&self) -> U128 {
        U128(STARTER_CLAIM_ATOMIC)
    }

    pub fn max_game_reward_amount(&self) -> U128 {
        U128(MAX_GAME_REWARD_ATOMIC)
    }

    pub fn has_claimed_starter(&self, account_id: AccountId) -> bool {
        self.starter_claimed.get(&account_id).unwrap_or(false)
    }

    pub fn is_reward_processed(&self, reward_id: String) -> bool {
        self.processed_reward_ids.get(&reward_id).unwrap_or(false)
    }

    /// One fixed demo starter allocation per registered account.
    ///
    /// 100 NUVE with decimals=4 = 1_000_000 atomic units.
    pub fn starter_claim(&mut self) -> U128 {
        let caller = env::predecessor_account_id();

        assert!(
            self.token.accounts.get(&caller).is_some(),
            "Account must be registered for FT storage"
        );
        assert!(
            !self.starter_claimed.get(&caller).unwrap_or(false),
            "Starter claim already used"
        );

        self.starter_claimed.insert(&caller, &true);
        self.token.internal_deposit(&caller, STARTER_CLAIM_ATOMIC);

        FtMint {
            owner_id: &caller,
            amount: U128(STARTER_CLAIM_ATOMIC),
            memo: Some("NUVI Hackathon starter claim"),
        }
        .emit();

        U128(STARTER_CLAIM_ATOMIC)
    }

    /// Idempotent reward minting for the Living NFT game contract.
    ///
    /// `reward_id` will later be derived from a unique settled action.
    /// Returning false means the reward was already processed and was not
    /// minted a second time.
    pub fn mint_game_reward(
        &mut self,
        receiver_id: AccountId,
        reward_id: String,
        amount: U128,
    ) -> bool {
        assert_eq!(
            env::predecessor_account_id(),
            self.reward_minter_id,
            "Caller is not reward minter"
        );

        assert!(!reward_id.is_empty(), "reward_id must not be empty");
        assert!(
            reward_id.len() <= MAX_REWARD_ID_BYTES,
            "reward_id is too long"
        );

        assert!(amount.0 > 0, "Reward amount must be positive");
        assert!(
            amount.0 <= MAX_GAME_REWARD_ATOMIC,
            "Reward exceeds per-call maximum"
        );

        assert!(
            self.token.accounts.get(&receiver_id).is_some(),
            "Receiver must be registered for FT storage"
        );

        if self.processed_reward_ids.get(&reward_id).unwrap_or(false) {
            return false;
        }

        self.processed_reward_ids.insert(&reward_id, &true);
        self.token.internal_deposit(&receiver_id, amount.0);

        FtMint {
            owner_id: &receiver_id,
            amount,
            memo: Some("NUVI gameplay reward"),
        }
        .emit();

        true
    }

    /// Burns caller-owned NUVE.
    ///
    /// This is intentionally general: later the Living NFT contract can
    /// burn NUVE held by its own FT account after a successful Burn-routed
    /// payment.
    #[payable]
    pub fn burn(&mut self, amount: U128) -> U128 {
        assert_one_yocto();
        assert!(amount.0 > 0, "Burn amount must be positive");

        let caller = env::predecessor_account_id();

        self.token.internal_withdraw(&caller, amount.0);

        FtBurn {
            owner_id: &caller,
            amount,
            memo: Some("NUVI burn"),
        }
        .emit();

        amount
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().expect("Metadata is missing")
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.token.storage_unregister(force)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

/// Callback required by the standard FungibleToken helper's
/// `ft_transfer_call` implementation.
#[near]
impl Contract {
    #[private]
    pub fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, _) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);

        U128(used_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, AccountId};

    fn account(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn context(predecessor: &str, deposit: u128) {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(account("nuve-demo.testnet"))
            .predecessor_account_id(account(predecessor))
            .attached_deposit(NearToken::from_yoctonear(deposit));
        testing_env!(builder.build());
    }

    fn contract() -> Contract {
        context("owner.testnet", 0);

        Contract::new(account("owner.testnet"), account("living.testnet"))
    }

    fn register(contract: &mut Contract, account_id: &str) {
        contract
            .token
            .internal_register_account(&account(account_id));
    }

    #[test]
    fn metadata_and_zero_supply_are_correct() {
        let contract = contract();
        let metadata = contract.ft_metadata();

        assert_eq!(metadata.spec, FT_METADATA_SPEC);
        assert_eq!(metadata.name, "NUVI Demo Token");
        assert_eq!(metadata.symbol, "NUVE");
        assert_eq!(metadata.decimals, 4);
        assert_eq!(contract.ft_total_supply(), U128(0));
        assert_eq!(contract.starter_claim_amount(), U128(1_000_000));
    }

    #[test]
    fn starter_claim_mints_exactly_100_nuve_once() {
        let mut contract = contract();

        context("owner.testnet", 0);
        let amount = contract.starter_claim();

        assert_eq!(amount, U128(1_000_000));
        assert_eq!(
            contract.ft_balance_of(account("owner.testnet")),
            U128(1_000_000)
        );
        assert_eq!(contract.ft_total_supply(), U128(1_000_000));
        assert!(contract.has_claimed_starter(account("owner.testnet")));

        let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.starter_claim();
        }));

        assert!(duplicate.is_err());
        assert_eq!(contract.ft_total_supply(), U128(1_000_000));
    }

    #[test]
    fn starter_claim_requires_storage_registration() {
        let mut contract = contract();

        context("alice.testnet", 0);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.starter_claim();
        }));

        assert!(result.is_err());
        assert_eq!(contract.ft_balance_of(account("alice.testnet")), U128(0));
        assert_eq!(contract.ft_total_supply(), U128(0));
    }

    #[test]
    fn game_reward_is_authorized_and_idempotent() {
        let mut contract = contract();
        register(&mut contract, "alice.testnet");

        context("living.testnet", 0);

        assert!(contract.mint_game_reward(
            account("alice.testnet"),
            "work:alice:1".to_string(),
            U128(NUVE_UNIT),
        ));

        assert_eq!(
            contract.ft_balance_of(account("alice.testnet")),
            U128(NUVE_UNIT)
        );

        assert!(!contract.mint_game_reward(
            account("alice.testnet"),
            "work:alice:1".to_string(),
            U128(NUVE_UNIT),
        ));

        assert_eq!(
            contract.ft_balance_of(account("alice.testnet")),
            U128(NUVE_UNIT)
        );
        assert_eq!(contract.ft_total_supply(), U128(NUVE_UNIT));
    }

    #[test]
    fn unauthorized_account_cannot_mint_game_reward() {
        let mut contract = contract();
        register(&mut contract, "alice.testnet");

        context("alice.testnet", 0);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.mint_game_reward(
                account("alice.testnet"),
                "unauthorized:1".to_string(),
                U128(NUVE_UNIT),
            );
        }));

        assert!(result.is_err());
        assert_eq!(contract.ft_total_supply(), U128(0));
    }

    #[test]
    fn reward_is_bounded_per_call() {
        let mut contract = contract();
        register(&mut contract, "alice.testnet");

        context("living.testnet", 0);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.mint_game_reward(
                account("alice.testnet"),
                "oversized:1".to_string(),
                U128(MAX_GAME_REWARD_ATOMIC + 1),
            );
        }));

        assert!(result.is_err());
        assert_eq!(contract.ft_total_supply(), U128(0));
        assert!(!contract.is_reward_processed("oversized:1".to_string()));
    }

    #[test]
    fn burn_reduces_balance_and_total_supply() {
        let mut contract = contract();

        context("owner.testnet", 0);
        contract.starter_claim();

        context("owner.testnet", 1);
        let burned = contract.burn(U128(25 * NUVE_UNIT));

        assert_eq!(burned, U128(25 * NUVE_UNIT));
        assert_eq!(
            contract.ft_balance_of(account("owner.testnet")),
            U128(75 * NUVE_UNIT)
        );
        assert_eq!(contract.ft_total_supply(), U128(75 * NUVE_UNIT));
    }

    #[test]
    fn standard_transfer_works_between_registered_accounts() {
        let mut contract = contract();
        register(&mut contract, "alice.testnet");

        context("owner.testnet", 0);
        contract.starter_claim();

        context("owner.testnet", 1);
        contract.ft_transfer(account("alice.testnet"), U128(10 * NUVE_UNIT), None);

        assert_eq!(
            contract.ft_balance_of(account("owner.testnet")),
            U128(90 * NUVE_UNIT)
        );
        assert_eq!(
            contract.ft_balance_of(account("alice.testnet")),
            U128(10 * NUVE_UNIT)
        );
        assert_eq!(contract.ft_total_supply(), U128(100 * NUVE_UNIT));
    }
}
