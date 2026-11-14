pub mod events;
pub mod gameplay;
pub mod living;
pub mod types;

use borsh::{BorshDeserialize, BorshSerialize};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::non_fungible_token::metadata::NFTContractMetadata;
use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, near, near_bindgen, AccountId, BorshStorageKey, Gas, NearToken,
    PanicOnDefault, Promise, PromiseOrValue,
};
use std::collections::{HashMap, HashSet};

pub use crate::gameplay::*;
pub use crate::living::*;
pub use crate::types::*;

const STATE_VERSION_V2: u32 = 2;
const STATE_VERSION_V3: u32 = 3;
const STATE_VERSION_V4: u32 = 4;
const STATE_VERSION_V5: u32 = 5;
const STATE_VERSION_V6: u32 = 6;
const STATE_VERSION_V7: u32 = 7;
const MAX_CLASS_ID_BYTES: usize = 64;
const BPS_DENOMINATOR: u32 = 10_000;
const GAS_FOR_NFT_ON_TRANSFER: Gas = Gas::from_tgas(30);
const GAS_FOR_NFT_RESOLVE_TRANSFER: Gas = Gas::from_tgas(10);
const GAS_FOR_NFT_ON_APPROVE: Gas = Gas::from_tgas(10);
const GAS_FOR_NUVE_ROUTE: Gas = Gas::from_tgas(20);
const GAS_FOR_LIVING_PAYMENT_CALLBACK: Gas = Gas::from_tgas(20);
const GAS_FOR_GAME_REWARD: Gas = Gas::from_tgas(20);
const GAS_FOR_FEED_REWARD_CALLBACK: Gas = Gas::from_tgas(20);
const GAS_FOR_NUVI_ACCOUNT_INIT: Gas = Gas::from_tgas(20);
const GAS_FOR_NUVI_ACCOUNT_CALLBACK: Gas = Gas::from_tgas(20);
const GAS_FOR_NUVE_STORAGE_DEPOSIT: Gas = Gas::from_tgas(20);
const GAS_FOR_NUVI_ACCOUNT_FINAL_CALLBACK: Gas = Gas::from_tgas(20);
const MIN_STORAGE_BYTES: u128 = 1_000;
const NUVI_ACCOUNT_OPERATIONAL_RESERVE_YOCTO: u128 = 100_000_000_000_000_000_000_000;
const NUVE_STORAGE_DEPOSIT_YOCTO: u128 = 1_250_000_000_000_000_000_000;
const NUVI_ACCOUNT_DEMO_REWARD_ATOMIC: u128 = 100 * NUVE_UNIT;
const NUVI_ACCOUNT_WASM: &[u8] = include_bytes!("../res/nuvi_account.wasm");

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Metadata,
    TokensById,
    TokenIds,
    TokensPerOwner,
    TokensPerOwnerInner { account_hash: Vec<u8> },
    TokenMetadataById,
    ApprovalsById,
    NextApprovalIdById,
    RoyaltiesById,
    Admins,
    PermissionsByAccount,
    StorageDeposits,
    ClassMaxSupplyById,
    TokenClassById,
    ClassTokenBySerial,
    LivingStateById,
    PendingLivingPaymentByToken,
    StarterClaimedByAccount,
}

#[ext_contract(ext_nft_receiver)]
#[allow(dead_code)]
trait NftReceiver {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool>;
}

#[ext_contract(ext_nft_approval_receiver)]
#[allow(dead_code)]
trait NftApprovalReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: ApprovalId,
        msg: String,
    );
}

#[ext_contract(ext_self)]
#[allow(dead_code)]
trait NftResolver {
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, ApprovalId>>,
    ) -> bool;

    fn on_living_payment_routed(
        &mut self,
        token_id: TokenId,
        settlement_id: u64,
        amount: U128,
    ) -> U128;

    fn on_feed_reward_minted(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        ready_at_ns: u64,
    ) -> bool;

    fn on_nuvi_account_created(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_nuvi_account_storage_registered(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> PromiseOrValue<bool>;

    fn on_nuvi_account_reward_minted(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> bool;
}

#[ext_contract(ext_nuve_ft)]
#[allow(dead_code)]
trait NuveFt {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn burn(&mut self, amount: U128) -> U128;
    fn mint_game_reward(&mut self, receiver_id: AccountId, reward_id: String, amount: U128)
        -> bool;
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
}

#[derive(BorshDeserialize, BorshSerialize)]
struct OldContractPrefix {
    owner_id: AccountId,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractV2 {
    owner_id: AccountId,
    proposed_owner_id: Option<AccountId>,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
    tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    next_token_id: u64,
    admins: UnorderedSet<AccountId>,
    permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pause_flags: PauseFlags,
    config: ContractConfig,
    storage_deposits: LookupMap<AccountId, u128>,
    state_version: u32,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractV3 {
    owner_id: AccountId,
    proposed_owner_id: Option<AccountId>,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
    tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    next_token_id: u64,
    admins: UnorderedSet<AccountId>,
    permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pause_flags: PauseFlags,
    config: ContractConfig,
    storage_deposits: LookupMap<AccountId, u128>,
    state_version: u32,
    class_max_supply_by_id: LookupMap<ClassId, u32>,
    token_class_by_id: LookupMap<TokenId, TokenClass>,
    class_token_by_serial: LookupMap<(ClassId, u32), TokenId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractV4 {
    owner_id: AccountId,
    proposed_owner_id: Option<AccountId>,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
    tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    next_token_id: u64,
    admins: UnorderedSet<AccountId>,
    permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pause_flags: PauseFlags,
    config: ContractConfig,
    storage_deposits: LookupMap<AccountId, u128>,
    state_version: u32,
    class_max_supply_by_id: LookupMap<ClassId, u32>,
    token_class_by_id: LookupMap<TokenId, TokenClass>,
    class_token_by_serial: LookupMap<(ClassId, u32), TokenId>,
    living_state_by_id: LookupMap<TokenId, LivingState>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractV5 {
    owner_id: AccountId,
    proposed_owner_id: Option<AccountId>,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
    tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    next_token_id: u64,
    admins: UnorderedSet<AccountId>,
    permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pause_flags: PauseFlags,
    config: ContractConfig,
    storage_deposits: LookupMap<AccountId, u128>,
    state_version: u32,
    class_max_supply_by_id: LookupMap<ClassId, u32>,
    token_class_by_id: LookupMap<TokenId, TokenClass>,
    class_token_by_serial: LookupMap<(ClassId, u32), TokenId>,
    living_state_by_id: LookupMap<TokenId, LivingState>,
    nuve_ft_id: AccountId,
    treasury_id: AccountId,
    pending_living_payment_by_token: LookupMap<TokenId, PendingLivingPayment>,
    next_living_settlement_id: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractV6 {
    owner_id: AccountId,
    proposed_owner_id: Option<AccountId>,
    metadata: LazyOption<NFTContractMetadata>,
    tokens_by_id: LookupMap<TokenId, Token>,
    token_ids: UnorderedSet<TokenId>,
    tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    next_token_id: u64,
    admins: UnorderedSet<AccountId>,
    permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pause_flags: PauseFlags,
    config: ContractConfig,
    storage_deposits: LookupMap<AccountId, u128>,
    state_version: u32,
    class_max_supply_by_id: LookupMap<ClassId, u32>,
    token_class_by_id: LookupMap<TokenId, TokenClass>,
    class_token_by_serial: LookupMap<(ClassId, u32), TokenId>,
    living_state_by_id: LookupMap<TokenId, LivingState>,
    nuve_ft_id: AccountId,
    treasury_id: AccountId,
    pending_living_payment_by_token: LookupMap<TokenId, PendingLivingPayment>,
    next_living_settlement_id: u64,
    global_treasury_total: u128,
    global_burn_total: u128,
}

#[near(
    contract_state,
    contract_metadata(
        version = "7.0.0",
        link = "https://nuvelor.gg",
        standard(standard = "nep330", version = "1.1.0"),
        standard(standard = "nep171", version = "1.0.0"),
        standard(standard = "nep177", version = "2.0.0"),
        standard(standard = "nep178", version = "1.0.0"),
        standard(standard = "nep181", version = "1.0.0"),
        standard(standard = "nep199", version = "1.0.0"),
        standard(standard = "nep256", version = "1.0.0"),
        standard(standard = "nep297", version = "1.0.0"),
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner_id: AccountId,
    pub proposed_owner_id: Option<AccountId>,
    pub metadata: LazyOption<NFTContractMetadata>,
    pub tokens_by_id: LookupMap<TokenId, Token>,
    pub token_ids: UnorderedSet<TokenId>,
    pub tokens_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>,
    pub token_metadata_by_id: LookupMap<TokenId, TokenMetadata>,
    pub approvals_by_id: LookupMap<TokenId, HashMap<AccountId, ApprovalId>>,
    pub next_approval_id_by_id: LookupMap<TokenId, ApprovalId>,
    pub royalties_by_id: LookupMap<TokenId, RoyaltyConfig>,
    pub next_token_id: u64,
    pub admins: UnorderedSet<AccountId>,
    pub permissions_by_account: LookupMap<AccountId, PermissionSet>,
    pub pause_flags: PauseFlags,
    pub config: ContractConfig,
    pub storage_deposits: LookupMap<AccountId, u128>,
    pub state_version: u32,
    pub class_max_supply_by_id: LookupMap<ClassId, u32>,
    pub token_class_by_id: LookupMap<TokenId, TokenClass>,
    pub class_token_by_serial: LookupMap<(ClassId, u32), TokenId>,
    pub living_state_by_id: LookupMap<TokenId, LivingState>,
    pub nuve_ft_id: AccountId,
    pub treasury_id: AccountId,
    pub pending_living_payment_by_token: LookupMap<TokenId, PendingLivingPayment>,
    pub next_living_settlement_id: u64,
    pub global_treasury_total: u128,
    pub global_burn_total: u128,
    pub starter_claimed_by_account: LookupMap<AccountId, bool>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(config: ContractConfigInput, nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Contract is already initialized");
        Self::new_clean(config, nuve_ft_id, treasury_id)
    }

    #[init(ignore_state)]
    pub fn migrate_empty_state(
        config: ContractConfigInput,
        nuve_ft_id: AccountId,
        treasury_id: AccountId,
    ) -> Self {
        let data = env::storage_read(b"STATE").expect("State is not initialized");
        let mut slice = data.as_slice();
        let old = OldContractPrefix::deserialize(&mut slice)
            .unwrap_or_else(|_| env::panic_str("Cannot deserialize old contract state"));
        let predecessor = env::predecessor_account_id();
        assert_eq!(
            predecessor, old.owner_id,
            "Caller is not authorized to migrate"
        );
        assert_eq!(old.token_ids.len(), 0, "Old total supply must be zero");
        Self::new_clean(config, nuve_ft_id, treasury_id)
    }

    #[init(ignore_state)]
    pub fn migrate_v2_to_v7(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
        let old: ContractV2 = env::state_read()
            .unwrap_or_else(|| env::panic_str("Cannot deserialize V2 contract state"));

        assert_eq!(old.state_version, STATE_VERSION_V2, "Expected V2 state");
        assert_eq!(
            env::predecessor_account_id(),
            old.owner_id,
            "Caller is not authorized to migrate"
        );

        let mut contract = Self {
            owner_id: old.owner_id,
            proposed_owner_id: old.proposed_owner_id,
            metadata: old.metadata,
            tokens_by_id: old.tokens_by_id,
            token_ids: old.token_ids,
            tokens_per_owner: old.tokens_per_owner,
            token_metadata_by_id: old.token_metadata_by_id,
            approvals_by_id: old.approvals_by_id,
            next_approval_id_by_id: old.next_approval_id_by_id,
            royalties_by_id: old.royalties_by_id,
            next_token_id: old.next_token_id,
            admins: old.admins,
            permissions_by_account: old.permissions_by_account,
            pause_flags: old.pause_flags,
            config: old.config,
            storage_deposits: old.storage_deposits,
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: LookupMap::new(StorageKey::ClassMaxSupplyById),
            token_class_by_id: LookupMap::new(StorageKey::TokenClassById),
            class_token_by_serial: LookupMap::new(StorageKey::ClassTokenBySerial),
            living_state_by_id: LookupMap::new(StorageKey::LivingStateById),
            nuve_ft_id,
            treasury_id,
            pending_living_payment_by_token: LookupMap::new(
                StorageKey::PendingLivingPaymentByToken,
            ),
            next_living_settlement_id: 1,
            global_treasury_total: 0,
            global_burn_total: 0,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        };

        contract.initialize_living_states_for_existing_tokens();
        contract
    }

    #[init(ignore_state)]
    pub fn migrate_v3_to_v7(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
        let old: ContractV3 = env::state_read()
            .unwrap_or_else(|| env::panic_str("Cannot deserialize V3 contract state"));

        assert_eq!(old.state_version, STATE_VERSION_V3, "Expected V3 state");
        assert_eq!(
            env::predecessor_account_id(),
            old.owner_id,
            "Caller is not authorized to migrate"
        );

        let mut contract = Self {
            owner_id: old.owner_id,
            proposed_owner_id: old.proposed_owner_id,
            metadata: old.metadata,
            tokens_by_id: old.tokens_by_id,
            token_ids: old.token_ids,
            tokens_per_owner: old.tokens_per_owner,
            token_metadata_by_id: old.token_metadata_by_id,
            approvals_by_id: old.approvals_by_id,
            next_approval_id_by_id: old.next_approval_id_by_id,
            royalties_by_id: old.royalties_by_id,
            next_token_id: old.next_token_id,
            admins: old.admins,
            permissions_by_account: old.permissions_by_account,
            pause_flags: old.pause_flags,
            config: old.config,
            storage_deposits: old.storage_deposits,
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: old.class_max_supply_by_id,
            token_class_by_id: old.token_class_by_id,
            class_token_by_serial: old.class_token_by_serial,
            living_state_by_id: LookupMap::new(StorageKey::LivingStateById),
            nuve_ft_id,
            treasury_id,
            pending_living_payment_by_token: LookupMap::new(
                StorageKey::PendingLivingPaymentByToken,
            ),
            next_living_settlement_id: 1,
            global_treasury_total: 0,
            global_burn_total: 0,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        };

        contract.initialize_living_states_for_existing_tokens();
        contract
    }

    #[init(ignore_state)]
    pub fn migrate_v4_to_v7(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
        let old: ContractV4 = env::state_read()
            .unwrap_or_else(|| env::panic_str("Cannot deserialize V4 contract state"));

        assert_eq!(old.state_version, STATE_VERSION_V4, "Expected V4 state");
        assert_eq!(
            env::predecessor_account_id(),
            old.owner_id,
            "Caller is not authorized to migrate"
        );

        Self {
            owner_id: old.owner_id,
            proposed_owner_id: old.proposed_owner_id,
            metadata: old.metadata,
            tokens_by_id: old.tokens_by_id,
            token_ids: old.token_ids,
            tokens_per_owner: old.tokens_per_owner,
            token_metadata_by_id: old.token_metadata_by_id,
            approvals_by_id: old.approvals_by_id,
            next_approval_id_by_id: old.next_approval_id_by_id,
            royalties_by_id: old.royalties_by_id,
            next_token_id: old.next_token_id,
            admins: old.admins,
            permissions_by_account: old.permissions_by_account,
            pause_flags: old.pause_flags,
            config: old.config,
            storage_deposits: old.storage_deposits,
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: old.class_max_supply_by_id,
            token_class_by_id: old.token_class_by_id,
            class_token_by_serial: old.class_token_by_serial,
            living_state_by_id: old.living_state_by_id,
            nuve_ft_id,
            treasury_id,
            pending_living_payment_by_token: LookupMap::new(
                StorageKey::PendingLivingPaymentByToken,
            ),
            next_living_settlement_id: 1,
            global_treasury_total: 0,
            global_burn_total: 0,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        }
    }

    #[init(ignore_state)]
    pub fn migrate_v5_to_v7() -> Self {
        let old: ContractV5 = env::state_read()
            .unwrap_or_else(|| env::panic_str("Cannot deserialize V5 contract state"));

        assert_eq!(old.state_version, STATE_VERSION_V5, "Expected V5 state");
        assert_eq!(
            env::predecessor_account_id(),
            old.owner_id,
            "Caller is not authorized to migrate"
        );

        Self {
            owner_id: old.owner_id,
            proposed_owner_id: old.proposed_owner_id,
            metadata: old.metadata,
            tokens_by_id: old.tokens_by_id,
            token_ids: old.token_ids,
            tokens_per_owner: old.tokens_per_owner,
            token_metadata_by_id: old.token_metadata_by_id,
            approvals_by_id: old.approvals_by_id,
            next_approval_id_by_id: old.next_approval_id_by_id,
            royalties_by_id: old.royalties_by_id,
            next_token_id: old.next_token_id,
            admins: old.admins,
            permissions_by_account: old.permissions_by_account,
            pause_flags: old.pause_flags,
            config: old.config,
            storage_deposits: old.storage_deposits,
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: old.class_max_supply_by_id,
            token_class_by_id: old.token_class_by_id,
            class_token_by_serial: old.class_token_by_serial,
            living_state_by_id: old.living_state_by_id,
            nuve_ft_id: old.nuve_ft_id,
            treasury_id: old.treasury_id,
            pending_living_payment_by_token: old.pending_living_payment_by_token,
            next_living_settlement_id: old.next_living_settlement_id,
            global_treasury_total: 0,
            global_burn_total: 0,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        }
    }

    #[init(ignore_state)]
    pub fn migrate_v6_to_v7() -> Self {
        let old: ContractV6 = env::state_read()
            .unwrap_or_else(|| env::panic_str("Cannot deserialize V6 contract state"));

        assert_eq!(old.state_version, STATE_VERSION_V6, "Expected V6 state");
        assert_eq!(
            env::predecessor_account_id(),
            old.owner_id,
            "Caller is not authorized to migrate"
        );

        Self {
            owner_id: old.owner_id,
            proposed_owner_id: old.proposed_owner_id,
            metadata: old.metadata,
            tokens_by_id: old.tokens_by_id,
            token_ids: old.token_ids,
            tokens_per_owner: old.tokens_per_owner,
            token_metadata_by_id: old.token_metadata_by_id,
            approvals_by_id: old.approvals_by_id,
            next_approval_id_by_id: old.next_approval_id_by_id,
            royalties_by_id: old.royalties_by_id,
            next_token_id: old.next_token_id,
            admins: old.admins,
            permissions_by_account: old.permissions_by_account,
            pause_flags: old.pause_flags,
            config: old.config,
            storage_deposits: old.storage_deposits,
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: old.class_max_supply_by_id,
            token_class_by_id: old.token_class_by_id,
            class_token_by_serial: old.class_token_by_serial,
            living_state_by_id: old.living_state_by_id,
            nuve_ft_id: old.nuve_ft_id,
            treasury_id: old.treasury_id,
            pending_living_payment_by_token: old.pending_living_payment_by_token,
            next_living_settlement_id: old.next_living_settlement_id,
            global_treasury_total: old.global_treasury_total,
            global_burn_total: old.global_burn_total,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        }
    }

    pub fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().expect("Metadata is missing")
    }

    pub fn get_config(&self) -> ContractConfig {
        self.config.clone()
    }

    pub fn get_pause_flags(&self) -> PauseFlags {
        self.pause_flags.clone()
    }

    #[payable]
    pub fn update_contract_metadata(
        &mut self,
        metadata: NFTContractMetadata,
    ) -> NFTContractMetadata {
        self.assert_one_yocto();
        self.assert_owner();
        Self::validate_contract_metadata(&metadata);
        self.metadata.set(&metadata);
        events::log_event(
            "contract_metadata_updated",
            serde_json::json!({ "metadata": metadata.clone() }),
        );
        metadata
    }

    #[payable]
    pub fn update_token_metadata(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
    ) -> TokenView {
        self.assert_one_yocto();
        self.assert_owner();
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        self.validate_metadata(&metadata);
        self.token_metadata_by_id.insert(&token_id, &metadata);
        events::log_event(
            "token_metadata_updated",
            serde_json::json!({
                "token_id": token_id,
                "metadata": metadata,
            }),
        );
        self.token_view(token)
    }

    #[payable]
    pub fn update_token_royalty(
        &mut self,
        token_id: TokenId,
        royalty: Option<RoyaltyConfig>,
    ) -> Option<RoyaltyConfig> {
        self.assert_one_yocto();
        self.assert_owner();
        self.tokens_by_id.get(&token_id).expect("Token not found");
        if let Some(config) = &royalty {
            self.validate_royalty(config);
            self.royalties_by_id.insert(&token_id, config);
        } else {
            self.royalties_by_id.remove(&token_id);
        }
        events::log_event(
            "token_royalty_updated",
            serde_json::json!({
                "token_id": token_id,
                "royalty": royalty.clone(),
            }),
        );
        royalty
    }

    #[payable]
    pub fn set_pause_flags(&mut self, flags: PauseFlags) {
        self.assert_one_yocto();
        self.assert_owner();
        self.pause_flags = flags.clone();
        events::log_event("pause_flags_updated", flags);
    }

    #[payable]
    pub fn update_config(&mut self, patch: ContractConfigPatch) -> ContractConfig {
        self.assert_one_yocto();
        self.assert_owner();
        self.config
            .apply_patch(patch, Self::computed_min_storage_deposit());
        Self::validate_config(&self.config);
        events::log_event(
            "config_updated",
            serde_json::json!({ "state_version": self.state_version }),
        );
        self.config.clone()
    }

    #[payable]
    pub fn propose_owner(&mut self, account_id: AccountId) {
        self.assert_one_yocto();
        self.assert_owner();
        self.proposed_owner_id = Some(account_id.clone());
        events::log_event(
            "owner_transfer_proposed",
            serde_json::json!({ "account_id": account_id }),
        );
    }

    #[payable]
    pub fn accept_owner(&mut self) {
        self.assert_one_yocto();
        let caller = env::predecessor_account_id();
        assert_eq!(
            self.proposed_owner_id.as_ref(),
            Some(&caller),
            "Caller is not proposed owner"
        );
        self.owner_id = caller.clone();
        self.proposed_owner_id = None;
        events::log_event(
            "owner_transfer_accepted",
            serde_json::json!({ "owner_id": caller }),
        );
    }

    #[payable]
    pub fn cancel_owner_transfer(&mut self) {
        self.assert_one_yocto();
        self.assert_owner();
        self.proposed_owner_id = None;
        events::log_event("owner_transfer_cancelled", serde_json::json!({}));
    }

    #[payable]
    pub fn add_admin(&mut self, account_id: AccountId) {
        self.assert_one_yocto();
        self.assert_owner();
        self.admins.insert(&account_id);
        events::log_event(
            "admin_added",
            serde_json::json!({ "account_id": account_id }),
        );
    }

    #[payable]
    pub fn remove_admin(&mut self, account_id: AccountId) {
        self.assert_one_yocto();
        self.assert_owner();
        self.admins.remove(&account_id);
        events::log_event(
            "admin_removed",
            serde_json::json!({ "account_id": account_id }),
        );
    }

    pub fn is_admin(&self, account_id: AccountId) -> bool {
        self.admins.contains(&account_id)
    }

    pub fn get_admins(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<AccountId> {
        self.paginate_set(&self.admins, from_index, limit)
    }

    #[payable]
    pub fn grant_permission(&mut self, account_id: AccountId, permission: Permission) {
        self.assert_one_yocto();
        self.assert_owner();
        let mut set = self
            .permissions_by_account
            .get(&account_id)
            .unwrap_or_default();
        if !set.contains(&permission) {
            set.push(permission);
            set.sort();
            self.permissions_by_account.insert(&account_id, &set);
        }
        events::log_event(
            "permission_granted",
            serde_json::json!({ "account_id": account_id, "permission": permission }),
        );
    }

    #[payable]
    pub fn revoke_permission(&mut self, account_id: AccountId, permission: Permission) {
        self.assert_one_yocto();
        self.assert_owner();
        let mut set = self
            .permissions_by_account
            .get(&account_id)
            .unwrap_or_default();
        set.retain(|item| *item != permission);
        if set.is_empty() {
            self.permissions_by_account.remove(&account_id);
        } else {
            self.permissions_by_account.insert(&account_id, &set);
        }
        events::log_event(
            "permission_revoked",
            serde_json::json!({ "account_id": account_id, "permission": permission }),
        );
    }

    pub fn has_permission(&self, account_id: AccountId, permission: Permission) -> bool {
        self.permissions_by_account
            .get(&account_id)
            .unwrap_or_default()
            .contains(&permission)
    }

    pub fn get_permissions(&self, account_id: AccountId) -> Vec<Permission> {
        let mut set = self
            .permissions_by_account
            .get(&account_id)
            .unwrap_or_default();
        set.sort();
        set
    }

    #[payable]
    pub fn create_token_class(&mut self, class_id: ClassId, max_supply: u32) {
        let caller = env::predecessor_account_id();
        assert!(
            self.is_owner_or_admin(&caller),
            "Caller is not authorized to create token class"
        );
        Self::validate_class_id(&class_id);
        assert!(max_supply > 0, "max_supply must be positive");
        assert!(
            self.class_max_supply_by_id.get(&class_id).is_none(),
            "Token class already exists"
        );
        let initial_storage = env::storage_usage();
        self.class_max_supply_by_id.insert(&class_id, &max_supply);
        self.settle_storage_cost(&caller, initial_storage, true);
    }

    pub fn get_token_class(&self, token_id: TokenId) -> Option<TokenClass> {
        self.token_class_by_id.get(&token_id)
    }

    pub fn get_token_class_max_supply(&self, class_id: ClassId) -> Option<u32> {
        self.class_max_supply_by_id.get(&class_id)
    }

    pub fn get_token_id_by_class_serial(&self, class_id: ClassId, serial: u32) -> Option<TokenId> {
        self.class_token_by_serial.get(&(class_id, serial))
    }

    pub fn get_living_state(&self, token_id: TokenId) -> Option<LivingStateView> {
        self.living_state_by_id
            .get(&token_id)
            .map(LivingStateView::from)
    }

    pub fn get_living_game_config(&self) -> LivingGameConfigView {
        LivingGameConfigView {
            nuve_ft_id: self.nuve_ft_id.clone(),
            treasury_id: self.treasury_id.clone(),
            nuve_decimals: NUVE_DECIMALS,
            feed_cost: U128(FEED_COST_ATOMIC),
            work_cost: U128(WORK_COST_ATOMIC),
            feed_duration_ns: FEED_DURATION_NS,
            work_duration_ns: WORK_DURATION_NS,
            mutation_threshold_xp: MUTATION_THRESHOLD_XP,
        }
    }

    pub fn get_global_alignment(&self) -> GlobalAlignmentView {
        let global_total = self
            .global_treasury_total
            .saturating_add(self.global_burn_total);
        let (treasury_share_bps, burn_share_bps) =
            global_alignment_bps(self.global_treasury_total, self.global_burn_total);

        GlobalAlignmentView {
            treasury_total: U128(self.global_treasury_total),
            burn_total: U128(self.global_burn_total),
            global_total: U128(global_total),
            treasury_share_bps,
            burn_share_bps,
        }
    }

    pub fn get_pending_living_payment(
        &self,
        token_id: TokenId,
    ) -> Option<PendingLivingPaymentView> {
        self.pending_living_payment_by_token
            .get(&token_id)
            .map(PendingLivingPaymentView::from)
    }

    pub fn get_mutation_preview(&self, token_id: TokenId) -> Option<MutationPreviewView> {
        let state = self.living_state_by_id.get(&token_id)?;
        let (form_a_weight, form_b_weight, sponsor_level, burner_level) = mutation_weights(&state);
        let (form_a_probability_bps, form_b_probability_bps) =
            mutation_probability_bps(form_a_weight, form_b_weight);

        Some(MutationPreviewView {
            token_id: token_id.clone(),
            eligible: state.form_id == BASIC_FORM_ID
                && state.mutation_count == 0
                && state.xp >= MUTATION_THRESHOLD_XP
                && state.active_action.is_none()
                && self
                    .pending_living_payment_by_token
                    .get(&token_id)
                    .is_none(),
            current_form_id: state.form_id,
            mutation_stage: state.mutation_count,
            xp: state.xp,
            threshold_xp: MUTATION_THRESHOLD_XP,
            work_a_xp: state.work_a_xp,
            work_b_xp: state.work_b_xp,
            sponsor_level,
            burner_level,
            form_a_weight,
            form_b_weight,
            form_a_probability_bps,
            form_b_probability_bps,
        })
    }

    pub fn expected_nuvi_account_id(&self, token_id: TokenId) -> AccountId {
        Self::derive_nuvi_account_id(&token_id)
    }

    pub fn nuvi_account_creation_deposit(&self) -> U128 {
        U128(Self::nuvi_account_required_deposit_yocto())
    }

    #[payable]
    pub fn provision_nuvi_account(&mut self, token_id: TokenId) -> AccountId {
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let caller = env::predecessor_account_id();
        assert_eq!(
            token.owner_id, caller,
            "Only current NFT owner may provision NUVI Account"
        );
        self.assert_living_token_unlocked(&token_id);

        let state = self
            .living_state_by_id
            .get(&token_id)
            .expect("Living state not found");
        assert!(
            state.mutation_count > 0,
            "NUVI must be mutated before account provisioning"
        );

        let metadata = self
            .token_metadata_by_id
            .get(&token_id)
            .expect("Token metadata not found");
        let existing_status = metadata
            .extra
            .as_deref()
            .and_then(|raw| near_sdk::serde_json::from_str::<near_sdk::serde_json::Value>(raw).ok())
            .and_then(|value| {
                value
                    .get("nuvi_account_status")
                    .and_then(|status| status.as_str())
                    .map(ToString::to_string)
            });
        assert!(
            existing_status.is_none(),
            "NUVI Account provisioning already started"
        );

        let required_deposit = Self::nuvi_account_required_deposit_yocto();
        let attached_deposit = env::attached_deposit().as_yoctonear();
        assert!(
            attached_deposit >= required_deposit,
            "Insufficient deposit for NUVI Account provisioning"
        );
        if attached_deposit > required_deposit {
            Promise::new(caller.clone())
                .transfer(NearToken::from_yoctonear(
                    attached_deposit - required_deposit,
                ))
                .detach();
        }

        let nuvi_account_id = Self::derive_nuvi_account_id(&token_id);
        self.set_nuvi_account_metadata_status(
            &token_id,
            "provisioning",
            Some(&nuvi_account_id),
            None,
            false,
        );

        let child_balance = Self::nuvi_account_child_balance_yocto();
        let init_args = near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
            "nft_contract_id": env::current_account_id(),
            "token_id": token_id,
            "nuve_ft_id": self.nuve_ft_id,
        }))
        .expect("NUVI Account init arguments must serialize");

        Promise::new(nuvi_account_id.clone())
            .create_account()
            .transfer(NearToken::from_yoctonear(child_balance))
            .deploy_contract(NUVI_ACCOUNT_WASM.to_vec())
            .function_call(
                "new".to_string(),
                init_args,
                NearToken::from_yoctonear(0),
                GAS_FOR_NUVI_ACCOUNT_INIT,
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_NUVI_ACCOUNT_CALLBACK)
                    .on_nuvi_account_created(token_id.clone(), nuvi_account_id.clone()),
            )
            .detach();

        events::log_event(
            "nuvi_account_provisioning_started",
            serde_json::json!({
                "token_id": token_id,
                "owner_id": caller,
                "nuvi_account_id": nuvi_account_id,
                "child_balance": U128(child_balance),
                "compatibility_provision": true,
            }),
        );

        nuvi_account_id
    }

    #[payable]
    pub fn mutate(&mut self, token_id: TokenId) -> LivingStateView {
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let caller = env::predecessor_account_id();
        assert_eq!(token.owner_id, caller, "Only current NFT owner may mutate");
        self.assert_living_token_unlocked(&token_id);

        let mut state = self
            .living_state_by_id
            .get(&token_id)
            .expect("Living state not found");
        assert_eq!(state.form_id, BASIC_FORM_ID, "Only Basic form may mutate");
        assert_eq!(state.mutation_count, 0, "MVP mutation already used");
        assert!(
            state.xp >= MUTATION_THRESHOLD_XP,
            "Mutation XP threshold is not reached"
        );

        let required_deposit = Self::nuvi_account_required_deposit_yocto();
        let attached_deposit = env::attached_deposit().as_yoctonear();
        assert!(
            attached_deposit >= required_deposit,
            "Insufficient deposit for NUVI Account provisioning"
        );
        if attached_deposit > required_deposit {
            Promise::new(caller.clone())
                .transfer(NearToken::from_yoctonear(
                    attached_deposit - required_deposit,
                ))
                .detach();
        }

        let nuvi_account_id = Self::derive_nuvi_account_id(&token_id);

        let (form_a_weight, form_b_weight, sponsor_level, burner_level) = mutation_weights(&state);
        let total_weight = form_a_weight
            .checked_add(form_b_weight)
            .expect("Mutation weight overflow");

        let mut entropy = Vec::with_capacity(96);
        entropy.extend_from_slice(&env::random_seed_array());
        entropy.extend_from_slice(token_id.as_bytes());
        entropy.extend_from_slice(&state.mutation_count.to_le_bytes());
        entropy.extend_from_slice(&env::block_height().to_le_bytes());
        entropy.extend_from_slice(&env::block_timestamp().to_le_bytes());
        let digest = env::sha256(&entropy);
        let roll_source = u64::from_le_bytes(
            digest[..8]
                .try_into()
                .expect("SHA-256 prefix must contain eight bytes"),
        );
        let roll = roll_source % total_weight;

        let selected_form_id = if roll < form_a_weight {
            FORM_A_ID
        } else {
            FORM_B_ID
        };

        state.form_id = selected_form_id.to_string();
        state.mutation_count = state
            .mutation_count
            .checked_add(1)
            .expect("Mutation count overflow");
        self.living_state_by_id.insert(&token_id, &state);
        self.set_nuvi_account_metadata_status(
            &token_id,
            "provisioning",
            Some(&nuvi_account_id),
            None,
            false,
        );

        let mutation_id = format!("{}:{}", token_id, state.mutation_count);
        events::log_event(
            "nuvi_mutated",
            serde_json::json!({
                "token_id": token_id,
                "owner_id": caller,
                "mutation_id": mutation_id,
                "mutation_stage": state.mutation_count,
                "form_id": state.form_id,
                "xp": state.xp,
                "work_a_xp": state.work_a_xp,
                "work_b_xp": state.work_b_xp,
                "sponsor_level": sponsor_level,
                "burner_level": burner_level,
                "form_a_weight": form_a_weight,
                "form_b_weight": form_b_weight,
                "roll": roll,
            }),
        );

        let child_balance = Self::nuvi_account_child_balance_yocto();
        let init_args = near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
            "nft_contract_id": env::current_account_id(),
            "token_id": token_id,
            "nuve_ft_id": self.nuve_ft_id,
        }))
        .expect("NUVI Account init arguments must serialize");

        Promise::new(nuvi_account_id.clone())
            .create_account()
            .transfer(NearToken::from_yoctonear(child_balance))
            .deploy_contract(NUVI_ACCOUNT_WASM.to_vec())
            .function_call(
                "new".to_string(),
                init_args,
                NearToken::from_yoctonear(0),
                GAS_FOR_NUVI_ACCOUNT_INIT,
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_NUVI_ACCOUNT_CALLBACK)
                    .on_nuvi_account_created(token_id.clone(), nuvi_account_id.clone()),
            )
            .detach();

        events::log_event(
            "nuvi_account_provisioning_started",
            serde_json::json!({
                "token_id": token_id,
                "owner_id": caller,
                "nuvi_account_id": nuvi_account_id,
                "child_balance": U128(child_balance),
                "executor_wasm_sha256": env::sha256(NUVI_ACCOUNT_WASM)
                    .iter()
                    .map(|byte| format!("{:02x}", byte))
                    .collect::<String>(),
            }),
        );

        LivingStateView::from(state)
    }

    #[private]
    pub fn on_nuvi_account_created(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> PromiseOrValue<bool> {
        if env::promise_result_checked(0, 4_096).is_err() {
            self.set_nuvi_account_metadata_status(
                &token_id,
                "failed",
                Some(&nuvi_account_id),
                Some("create_deploy_init"),
                false,
            );
            events::log_event(
                "nuvi_account_provisioning_failed",
                serde_json::json!({
                    "token_id": token_id,
                    "nuvi_account_id": nuvi_account_id,
                    "phase": "create_deploy_init",
                }),
            );
            return PromiseOrValue::Value(false);
        }

        PromiseOrValue::Promise(
            ext_nuve_ft::ext(self.nuve_ft_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(NUVE_STORAGE_DEPOSIT_YOCTO))
                .with_static_gas(GAS_FOR_NUVE_STORAGE_DEPOSIT)
                .storage_deposit(Some(nuvi_account_id.clone()), Some(true))
                .then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_NUVI_ACCOUNT_CALLBACK)
                        .on_nuvi_account_storage_registered(token_id, nuvi_account_id),
                ),
        )
    }

    #[private]
    pub fn on_nuvi_account_storage_registered(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> PromiseOrValue<bool> {
        if env::promise_result_checked(0, 4_096).is_err() {
            self.set_nuvi_account_metadata_status(
                &token_id,
                "failed",
                Some(&nuvi_account_id),
                Some("nuve_storage_registration"),
                false,
            );
            events::log_event(
                "nuvi_account_provisioning_failed",
                serde_json::json!({
                    "token_id": token_id,
                    "nuvi_account_id": nuvi_account_id,
                    "phase": "nuve_storage_registration",
                }),
            );
            return PromiseOrValue::Value(false);
        }

        let reward_id = format!("nuvi-account:{}:mutation:1", token_id);
        PromiseOrValue::Promise(
            ext_nuve_ft::ext(self.nuve_ft_id.clone())
                .with_static_gas(GAS_FOR_GAME_REWARD)
                .mint_game_reward(
                    nuvi_account_id.clone(),
                    reward_id,
                    U128(NUVI_ACCOUNT_DEMO_REWARD_ATOMIC),
                )
                .then(
                    ext_self::ext(env::current_account_id())
                        .with_static_gas(GAS_FOR_NUVI_ACCOUNT_FINAL_CALLBACK)
                        .on_nuvi_account_reward_minted(token_id, nuvi_account_id),
                ),
        )
    }

    #[private]
    pub fn on_nuvi_account_reward_minted(
        &mut self,
        token_id: TokenId,
        nuvi_account_id: AccountId,
    ) -> bool {
        if env::promise_result_checked(0, 4_096).is_err() {
            self.set_nuvi_account_metadata_status(
                &token_id,
                "failed",
                Some(&nuvi_account_id),
                Some("demo_reward"),
                false,
            );
            events::log_event(
                "nuvi_account_provisioning_failed",
                serde_json::json!({
                    "token_id": token_id,
                    "nuvi_account_id": nuvi_account_id,
                    "phase": "demo_reward",
                }),
            );
            return false;
        }

        let Some(state) = self.living_state_by_id.get(&token_id) else {
            return false;
        };
        if state.mutation_count == 0 {
            return false;
        }

        self.store_nuvi_account_id(&token_id, &nuvi_account_id);
        events::log_event(
            "nuvi_account_activated",
            serde_json::json!({
                "token_id": token_id,
                "nuvi_account_id": nuvi_account_id,
                "nuve_reward": U128(NUVI_ACCOUNT_DEMO_REWARD_ATOMIC),
            }),
        );
        true
    }

    pub fn complete_living_action(&mut self, token_id: TokenId) -> PromiseOrValue<bool> {
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        let caller = env::predecessor_account_id();
        assert_eq!(
            token.owner_id, caller,
            "Only current NFT owner may complete an action"
        );
        assert!(
            self.pending_living_payment_by_token
                .get(&token_id)
                .is_none(),
            "Living payment settlement is pending"
        );

        let mut state = self
            .living_state_by_id
            .get(&token_id)
            .expect("Living state not found");
        let action = state.active_action.expect("No active Living action");
        assert!(
            state.ready_at_ns > 0 && env::block_timestamp() >= state.ready_at_ns,
            "Living action is not ready"
        );

        match action {
            LivingActionKind::Feed => {
                let ready_at_ns = state.ready_at_ns;
                let reward_id = format!("feed:{}:{}", token_id, ready_at_ns);

                events::log_event(
                    "living_feed_reward_pending",
                    serde_json::json!({
                        "token_id": token_id,
                        "owner_id": caller,
                        "reward_id": reward_id,
                        "amount": U128(NUVE_UNIT),
                    }),
                );

                PromiseOrValue::Promise(
                    ext_nuve_ft::ext(self.nuve_ft_id.clone())
                        .with_static_gas(GAS_FOR_GAME_REWARD)
                        .mint_game_reward(caller.clone(), reward_id, U128(NUVE_UNIT))
                        .then(
                            ext_self::ext(env::current_account_id())
                                .with_static_gas(GAS_FOR_FEED_REWARD_CALLBACK)
                                .on_feed_reward_minted(token_id, caller, ready_at_ns),
                        ),
                )
            }
            LivingActionKind::WorkA | LivingActionKind::WorkB => {
                state.xp = state.xp.checked_add(1).expect("XP overflow");
                match action {
                    LivingActionKind::WorkA => {
                        state.work_a_xp =
                            state.work_a_xp.checked_add(1).expect("Work A XP overflow");
                    }
                    LivingActionKind::WorkB => {
                        state.work_b_xp =
                            state.work_b_xp.checked_add(1).expect("Work B XP overflow");
                    }
                    LivingActionKind::Feed => unreachable!(),
                }
                state.active_action = None;
                state.ready_at_ns = 0;
                self.living_state_by_id.insert(&token_id, &state);

                events::log_event(
                    "living_action_completed",
                    serde_json::json!({
                        "token_id": token_id,
                        "owner_id": caller,
                        "action": action,
                        "xp": state.xp,
                        "work_a_xp": state.work_a_xp,
                        "work_b_xp": state.work_b_xp,
                    }),
                );

                PromiseOrValue::Value(true)
            }
        }
    }

    #[private]
    pub fn on_feed_reward_minted(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        ready_at_ns: u64,
    ) -> bool {
        let reward_call_succeeded = env::promise_result_checked(0, 4_096).is_ok();
        if !reward_call_succeeded {
            events::log_event(
                "living_feed_reward_failed",
                serde_json::json!({
                    "token_id": token_id,
                    "owner_id": owner_id,
                    "ready_at_ns": ready_at_ns,
                }),
            );
            return false;
        }

        let Some(token) = self.tokens_by_id.get(&token_id) else {
            return false;
        };
        if token.owner_id != owner_id {
            return false;
        }

        let Some(mut state) = self.living_state_by_id.get(&token_id) else {
            return false;
        };
        if state.active_action != Some(LivingActionKind::Feed) || state.ready_at_ns != ready_at_ns {
            return false;
        }

        state.active_action = None;
        state.ready_at_ns = 0;
        self.living_state_by_id.insert(&token_id, &state);

        events::log_event(
            "living_action_completed",
            serde_json::json!({
                "token_id": token_id,
                "owner_id": owner_id,
                "action": LivingActionKind::Feed,
                "reward": U128(NUVE_UNIT),
            }),
        );

        true
    }

    #[private]
    pub fn on_living_payment_routed(
        &mut self,
        token_id: TokenId,
        settlement_id: u64,
        amount: U128,
    ) -> U128 {
        let Some(pending) = self.pending_living_payment_by_token.get(&token_id) else {
            events::log_event(
                "living_payment_callback_stale",
                serde_json::json!({
                    "token_id": token_id,
                    "settlement_id": settlement_id,
                    "amount": amount,
                }),
            );
            return U128(0);
        };

        if pending.settlement_id != settlement_id || pending.amount != amount.0 {
            events::log_event(
                "living_payment_callback_mismatch",
                serde_json::json!({
                    "token_id": token_id,
                    "settlement_id": settlement_id,
                    "expected_settlement_id": pending.settlement_id,
                    "amount": amount,
                    "expected_amount": U128(pending.amount),
                }),
            );
            return U128(0);
        }

        let route_succeeded = env::promise_result_checked(0, 4_096).is_ok();

        if !route_succeeded {
            self.pending_living_payment_by_token.remove(&token_id);
            events::log_event(
                "living_payment_refund",
                serde_json::json!({
                    "token_id": token_id,
                    "settlement_id": settlement_id,
                    "payer_id": pending.payer_id,
                    "action": pending.action,
                    "route": pending.route,
                    "amount": amount,
                }),
            );
            return amount;
        }

        match pending.route {
            EconomicRoute::Sponsor => {
                self.global_treasury_total = self
                    .global_treasury_total
                    .checked_add(pending.amount)
                    .expect("Global Treasury total overflow");
            }
            EconomicRoute::Burn => {
                self.global_burn_total = self
                    .global_burn_total
                    .checked_add(pending.amount)
                    .expect("Global Burn total overflow");
            }
        }

        let alignment = self.get_global_alignment();
        events::log_event(
            "global_alignment_updated",
            serde_json::json!({
                "treasury_total": alignment.treasury_total,
                "burn_total": alignment.burn_total,
                "global_total": alignment.global_total,
                "treasury_share_bps": alignment.treasury_share_bps,
                "burn_share_bps": alignment.burn_share_bps,
            }),
        );

        let Some(mut state) = self.living_state_by_id.get(&token_id) else {
            self.pending_living_payment_by_token.remove(&token_id);
            events::log_event(
                "living_payment_orphaned",
                serde_json::json!({
                    "token_id": token_id,
                    "settlement_id": settlement_id,
                    "payer_id": pending.payer_id,
                    "action": pending.action,
                    "route": pending.route,
                    "amount": amount,
                }),
            );
            return U128(0);
        };

        if state.active_action.is_some() {
            self.pending_living_payment_by_token.remove(&token_id);
            events::log_event(
                "living_payment_state_conflict",
                serde_json::json!({
                    "token_id": token_id,
                    "settlement_id": settlement_id,
                    "action": pending.action,
                    "route": pending.route,
                    "amount": amount,
                }),
            );
            return U128(0);
        }

        match pending.route {
            EconomicRoute::Sponsor => {
                state.sponsor_spent = state.sponsor_spent.saturating_add(pending.amount);
            }
            EconomicRoute::Burn => {
                state.burner_spent = state.burner_spent.saturating_add(pending.amount);
            }
        }

        match pending.action {
            LivingActionKind::Feed => {
                state.active_action = Some(LivingActionKind::Feed);
                state.ready_at_ns = env::block_timestamp()
                    .saturating_add(action_duration_ns(LivingActionKind::Feed));

                self.living_state_by_id.insert(&token_id, &state);
                self.pending_living_payment_by_token.remove(&token_id);

                events::log_event(
                    "living_action_started",
                    serde_json::json!({
                        "token_id": token_id,
                        "settlement_id": settlement_id,
                        "payer_id": pending.payer_id.clone(),
                        "action": pending.action,
                        "route": pending.route,
                        "amount": amount,
                        "ready_at_ns": state.ready_at_ns,
                        "immediate_completion": false,
                    }),
                );
            }
            LivingActionKind::WorkA | LivingActionKind::WorkB => {
                state.xp = state.xp.checked_add(1).expect("XP overflow");
                match pending.action {
                    LivingActionKind::WorkA => {
                        state.work_a_xp =
                            state.work_a_xp.checked_add(1).expect("Work A XP overflow");
                    }
                    LivingActionKind::WorkB => {
                        state.work_b_xp =
                            state.work_b_xp.checked_add(1).expect("Work B XP overflow");
                    }
                    LivingActionKind::Feed => unreachable!(),
                }
                state.active_action = None;
                state.ready_at_ns = 0;

                self.living_state_by_id.insert(&token_id, &state);
                self.pending_living_payment_by_token.remove(&token_id);

                events::log_event(
                    "living_action_started",
                    serde_json::json!({
                        "token_id": token_id,
                        "settlement_id": settlement_id,
                        "payer_id": pending.payer_id.clone(),
                        "action": pending.action,
                        "route": pending.route,
                        "amount": amount,
                        "ready_at_ns": 0,
                        "immediate_completion": true,
                    }),
                );
                events::log_event(
                    "living_action_completed",
                    serde_json::json!({
                        "token_id": token_id,
                        "settlement_id": settlement_id,
                        "owner_id": pending.payer_id,
                        "action": pending.action,
                        "xp": state.xp,
                        "work_a_xp": state.work_a_xp,
                        "work_b_xp": state.work_b_xp,
                        "immediate_completion": true,
                    }),
                );
            }
        }

        U128(0)
    }

    pub fn starter_claim_used(&self, account_id: AccountId) -> bool {
        self.starter_claimed_by_account
            .get(&account_id)
            .unwrap_or(false)
    }

    pub fn claim_starter_nuvi(&mut self) -> TokenView {
        assert!(!self.pause_flags.mint_paused, "Mint is paused");

        let caller = env::predecessor_account_id();
        assert!(
            !self
                .starter_claimed_by_account
                .get(&caller)
                .unwrap_or(false),
            "Starter NUVI claim already used"
        );

        let token_id = self.generate_next_token_id();
        let metadata = TokenMetadata {
            title: Some("NUVI Starter".to_string()),
            description: Some(
                "Nuvi is a mysterious creature powered by NUVELOR energy. It farms NUVE and evolves with the player."
                    .to_string(),
            ),
            media: None,
            media_hash: None,
            copies: Some(1),
            issued_at: Some(env::block_timestamp()),
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: Some(
                serde_json::json!({
                    "type": "Mascot",
                    "character": "Nuvi",
                    "base_rarity": "Common",
                    "form_id": BASIC_FORM_ID,
                })
                .to_string(),
            ),
            reference: None,
            reference_hash: None,
        };

        self.starter_claimed_by_account.insert(&caller, &true);
        let view = self.internal_mint(
            caller.clone(),
            token_id.clone(),
            metadata,
            None,
            Some("NUVI Hackathon starter claim".to_string()),
            None,
        );

        events::log_event(
            "starter_nuvi_claimed",
            serde_json::json!({
                "account_id": caller,
                "token_id": token_id,
                "base_rarity": "Common",
                "form_id": BASIC_FORM_ID,
            }),
        );

        view
    }

    #[payable]
    pub fn nft_mint(
        &mut self,
        receiver_id: AccountId,
        metadata: TokenMetadata,
        royalty: Option<RoyaltyConfig>,
        memo: Option<String>,
        class: Option<TokenClass>,
    ) -> TokenView {
        self.assert_can_mint();
        self.validate_mint_input(&metadata, royalty.as_ref(), class.as_ref());
        let initial_storage = env::storage_usage();
        let token_id = self.generate_next_token_id();
        let view = self.internal_mint(receiver_id, token_id, metadata, royalty, memo, class);
        self.settle_storage_cost(&env::predecessor_account_id(), initial_storage, true);
        view
    }

    #[payable]
    pub fn nft_batch_mint(
        &mut self,
        receiver_id: AccountId,
        tokens: Vec<MintTokenInput>,
        memo: Option<String>,
    ) -> Vec<TokenView> {
        self.assert_can_mint();
        assert!(
            tokens.len() <= self.config.max_batch as usize,
            "Batch size exceeds max_batch"
        );
        let mut batch_class_serials = HashSet::new();
        for item in &tokens {
            self.validate_mint_input(&item.metadata, item.royalty.as_ref(), item.class.as_ref());
            if let Some(class) = &item.class {
                assert!(
                    batch_class_serials.insert((class.class_id.clone(), class.serial)),
                    "Duplicate class serial in batch"
                );
            }
        }
        let initial_storage = env::storage_usage();
        let views = tokens
            .into_iter()
            .map(|item| {
                let token_id = self.generate_next_token_id();
                self.internal_mint(
                    receiver_id.clone(),
                    token_id,
                    item.metadata,
                    item.royalty,
                    memo.clone(),
                    item.class,
                )
            })
            .collect::<Vec<_>>();
        self.settle_storage_cost(&env::predecessor_account_id(), initial_storage, true);
        views
    }

    #[payable]
    pub fn nft_burn(&mut self, token_id: TokenId, memo: Option<String>) {
        self.assert_one_yocto();
        assert!(!self.pause_flags.burn_paused, "Burn is paused");
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        self.assert_can_burn(&token);
        self.assert_living_token_unlocked(&token_id);
        self.internal_burn(&token_id, memo);
    }

    #[payable]
    pub fn nft_batch_burn(&mut self, token_ids: Vec<TokenId>, memo: Option<String>) {
        self.assert_one_yocto();
        assert!(!self.pause_flags.burn_paused, "Burn is paused");
        assert!(
            token_ids.len() <= self.config.max_batch as usize,
            "Batch size exceeds max_batch"
        );
        let unique = token_ids.iter().collect::<HashSet<_>>();
        assert_eq!(
            unique.len(),
            token_ids.len(),
            "Duplicate token IDs in batch"
        );
        let caller = env::predecessor_account_id();
        let privileged =
            self.is_owner_or_admin(&caller) || self.account_has(&caller, Permission::Burn);
        for token_id in &token_ids {
            let token = self.tokens_by_id.get(token_id).expect("Token not found");
            if !privileged {
                assert_eq!(token.owner_id, caller, "Caller is not token owner");
            }
            self.assert_living_token_unlocked(token_id);
        }
        for token_id in token_ids {
            self.internal_burn(&token_id, memo.clone());
        }
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<TokenView> {
        self.tokens_by_id
            .get(&token_id)
            .map(|token| self.token_view(token))
    }

    #[payable]
    pub fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<ApprovalId>,
        memo: Option<String>,
    ) {
        self.assert_one_yocto();
        assert!(!self.pause_flags.transfers_paused, "Transfers are paused");
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        self.assert_approved_or_owner(&token, approval_id);
        self.assert_living_token_unlocked(&token_id);
        self.internal_transfer_unchecked(&token_id, &receiver_id, memo);
    }

    #[payable]
    pub fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<ApprovalId>,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        self.assert_one_yocto();
        assert!(!self.pause_flags.transfers_paused, "Transfers are paused");
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        self.assert_approved_or_owner(&token, approval_id);
        self.assert_living_token_unlocked(&token_id);
        let previous_owner_id = token.owner_id.clone();
        let approved_account_ids = Some(token.approved_account_ids.clone());
        self.internal_transfer_unchecked(&token_id, &receiver_id, memo);
        ext_nft_receiver::ext(receiver_id.clone())
            .with_static_gas(GAS_FOR_NFT_ON_TRANSFER)
            .nft_on_transfer(
                env::predecessor_account_id(),
                previous_owner_id.clone(),
                token_id.clone(),
                msg,
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_NFT_RESOLVE_TRANSFER)
                    .nft_resolve_transfer(
                        previous_owner_id,
                        receiver_id,
                        token_id,
                        approved_account_ids,
                    ),
            )
    }

    #[private]
    pub fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, ApprovalId>>,
    ) -> bool {
        let must_return = match env::promise_result_checked(0, 64) {
            Ok(value) => near_sdk::serde_json::from_slice::<bool>(&value)
                .map(|keep| !keep)
                .unwrap_or(true),
            _ => true,
        };
        if !must_return {
            return true;
        }
        let Some(mut token) = self.tokens_by_id.get(&token_id) else {
            return false;
        };
        if token.owner_id != receiver_id {
            return false;
        }
        self.internal_remove_token_from_owner(&receiver_id, &token_id);
        self.internal_add_token_to_owner(&previous_owner_id, &token_id);
        token.owner_id = previous_owner_id.clone();
        token.approved_account_ids = approved_account_ids.unwrap_or_default();
        if token.approved_account_ids.is_empty() {
            self.approvals_by_id.remove(&token_id);
        } else {
            self.approvals_by_id
                .insert(&token_id, &token.approved_account_ids);
        }
        self.tokens_by_id.insert(&token_id, &token);
        events::log_nft_transfer(
            &receiver_id,
            &previous_owner_id,
            vec![token_id],
            Some("resolve rollback".to_string()),
        );
        true
    }

    #[payable]
    pub fn nft_approve(
        &mut self,
        token_id: TokenId,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        assert!(!self.pause_flags.approvals_paused, "Approvals are paused");
        let attached = env::attached_deposit().as_yoctonear();
        assert!(attached > 0, "Requires attached deposit");
        let initial_storage = env::storage_usage();
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        assert_eq!(
            token.owner_id,
            env::predecessor_account_id(),
            "Caller is not token owner"
        );
        assert_ne!(account_id, token.owner_id, "Owner cannot approve self");
        assert!(
            token.approved_account_ids.len() < self.config.max_approvals_per_token as usize
                || token.approved_account_ids.contains_key(&account_id),
            "Approval limit exceeded"
        );
        let approval_id = self
            .next_approval_id_by_id
            .get(&token_id)
            .unwrap_or(0)
            .checked_add(1)
            .expect("Approval ID overflow");
        self.next_approval_id_by_id.insert(&token_id, &approval_id);
        token
            .approved_account_ids
            .insert(account_id.clone(), approval_id);
        self.approvals_by_id
            .insert(&token_id, &token.approved_account_ids);
        self.tokens_by_id.insert(&token_id, &token);
        self.refund_storage_deposit(&env::predecessor_account_id(), initial_storage);
        events::log_event(
            "nft_approved",
            serde_json::json!({
                "token_id": token_id.clone(),
                "owner_id": token.owner_id.clone(),
                "account_id": account_id.clone(),
                "approval_id": approval_id,
            }),
        );
        msg.map(|msg| {
            ext_nft_approval_receiver::ext(account_id)
                .with_static_gas(GAS_FOR_NFT_ON_APPROVE)
                .nft_on_approve(token_id, token.owner_id, approval_id, msg)
        })
    }

    #[payable]
    pub fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        self.assert_one_yocto();
        assert!(!self.pause_flags.approvals_paused, "Approvals are paused");
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        assert_eq!(
            token.owner_id,
            env::predecessor_account_id(),
            "Caller is not token owner"
        );
        let initial_storage = env::storage_usage();
        let owner_id = token.owner_id.clone();
        let removed = token.approved_account_ids.remove(&account_id);
        if token.approved_account_ids.is_empty() {
            self.approvals_by_id.remove(&token_id);
        } else {
            self.approvals_by_id
                .insert(&token_id, &token.approved_account_ids);
        }
        self.tokens_by_id.insert(&token_id, &token);
        self.refund_released_storage(&owner_id, initial_storage);
        if removed.is_some() {
            events::log_event(
                "nft_approval_revoked",
                serde_json::json!({
                    "token_id": token_id,
                    "owner_id": owner_id,
                    "account_id": account_id,
                }),
            );
        }
    }

    #[payable]
    pub fn nft_revoke_all(&mut self, token_id: TokenId) {
        self.assert_one_yocto();
        assert!(!self.pause_flags.approvals_paused, "Approvals are paused");
        let mut token = self.tokens_by_id.get(&token_id).expect("Token not found");
        assert_eq!(
            token.owner_id,
            env::predecessor_account_id(),
            "Caller is not token owner"
        );
        let initial_storage = env::storage_usage();
        let owner_id = token.owner_id.clone();
        let revoked_count = token.approved_account_ids.len();
        token.approved_account_ids.clear();
        self.approvals_by_id.remove(&token_id);
        self.tokens_by_id.insert(&token_id, &token);
        self.refund_released_storage(&owner_id, initial_storage);
        if revoked_count > 0 {
            events::log_event(
                "nft_approvals_revoked_all",
                serde_json::json!({
                    "token_id": token_id,
                    "owner_id": owner_id,
                    "revoked_count": revoked_count,
                }),
            );
        }
    }

    pub fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<ApprovalId>,
    ) -> bool {
        let Some(token) = self.tokens_by_id.get(&token_id) else {
            return false;
        };
        let Some(actual) = token.approved_account_ids.get(&approved_account_id) else {
            return false;
        };
        approval_id.map(|id| id == *actual).unwrap_or(true)
    }

    pub fn nft_total_supply(&self) -> U128 {
        U128(self.token_ids.len() as u128)
    }

    pub fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<TokenView> {
        let limit = self.checked_view_limit(limit);
        self.token_ids
            .iter()
            .skip(from_index.unwrap_or(U128(0)).0 as usize)
            .take(limit as usize)
            .filter_map(|token_id| self.nft_token(token_id))
            .collect()
    }

    pub fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        U128(
            self.tokens_per_owner
                .get(&account_id)
                .map(|set| set.len() as u128)
                .unwrap_or(0),
        )
    }

    pub fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<TokenView> {
        let limit = self.checked_view_limit(limit);
        self.tokens_per_owner
            .get(&account_id)
            .map(|set| {
                set.iter()
                    .skip(from_index.unwrap_or(U128(0)).0 as usize)
                    .take(limit as usize)
                    .filter_map(|token_id| self.nft_token(token_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: u32) -> Payout {
        self.calculate_payout(&token_id, balance, max_len_payout)
    }

    #[payable]
    pub fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<ApprovalId>,
        balance: U128,
        max_len_payout: u32,
    ) -> Payout {
        self.assert_one_yocto();
        assert!(!self.pause_flags.transfers_paused, "Transfers are paused");
        let token = self.tokens_by_id.get(&token_id).expect("Token not found");
        self.assert_approved_or_owner(&token, approval_id);
        self.assert_living_token_unlocked(&token_id);
        let payout = self.calculate_payout(&token_id, balance, max_len_payout);
        self.internal_transfer_unchecked(&token_id, &receiver_id, None);
        payout
    }

    #[payable]
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);
        let deposit = env::attached_deposit().as_yoctonear();
        let min = self.storage_balance_bounds().min.as_yoctonear();
        let current = self.storage_deposits.get(&account_id).unwrap_or(0);
        let required = min.saturating_sub(current);
        assert!(deposit >= required, "Insufficient attached deposit");
        let accepted = if registration_only.unwrap_or(false) {
            required
        } else {
            deposit
        };
        self.storage_deposits
            .insert(&account_id, &(current + accepted));
        self.refund_deposit(&env::predecessor_account_id(), deposit - accepted);
        self.storage_balance_of(account_id)
            .expect("Storage balance not found")
    }

    #[payable]
    pub fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        self.assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let min = self.storage_balance_bounds().min.as_yoctonear();
        let current = self.storage_deposits.get(&account_id).unwrap_or(0);
        assert!(current >= min, "Account is not registered");
        let available = current - min;
        let requested = amount.map(|value| value.0).unwrap_or(available);
        assert!(requested <= available, "The amount is too large");
        self.storage_deposits
            .insert(&account_id, &(current - requested));
        self.refund_deposit(&account_id, requested);
        self.storage_balance_of(account_id)
            .expect("Storage balance not found")
    }

    #[payable]
    pub fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.assert_one_yocto();
        assert!(!force.unwrap_or(false), "force=true is not supported");
        let account_id = env::predecessor_account_id();
        assert_eq!(
            self.nft_supply_for_owner(account_id.clone()).0,
            0,
            "Account has tokens"
        );
        if let Some(balance) = self.storage_deposits.remove(&account_id) {
            self.refund_deposit(&account_id, balance);
            true
        } else {
            false
        }
    }

    pub fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: NearToken::from_yoctonear(self.minimum_storage_deposit()),
            max: None,
        }
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.storage_deposits.get(&account_id).map(|total| {
            let min = self.storage_balance_bounds().min.as_yoctonear();
            StorageBalance {
                total: NearToken::from_yoctonear(total),
                available: NearToken::from_yoctonear(total.saturating_sub(min)),
            }
        })
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_eq!(
            env::predecessor_account_id(),
            self.nuve_ft_id,
            "Only configured NUVE FT is accepted"
        );

        let request: LivingPaymentMessage = near_sdk::serde_json::from_str(&msg)
            .unwrap_or_else(|_| env::panic_str("Invalid Living payment message"));

        let token = self
            .tokens_by_id
            .get(&request.token_id)
            .expect("Token not found");
        assert_eq!(
            token.owner_id, sender_id,
            "Only current NFT owner may pay for an action"
        );

        let state = self
            .living_state_by_id
            .get(&request.token_id)
            .expect("Living state not found");
        assert!(
            state.active_action.is_none(),
            "Living action is already active"
        );
        assert!(
            self.pending_living_payment_by_token
                .get(&request.token_id)
                .is_none(),
            "Living payment settlement is already pending"
        );

        let expected_amount = action_cost_atomic(request.action);
        assert_eq!(
            amount.0, expected_amount,
            "Incorrect NUVE amount for Living action"
        );

        let settlement_id = self.next_living_settlement_id;
        self.next_living_settlement_id = self
            .next_living_settlement_id
            .checked_add(1)
            .expect("Living settlement ID overflow");

        let pending = PendingLivingPayment {
            settlement_id,
            token_id: request.token_id.clone(),
            payer_id: sender_id,
            action: request.action,
            route: request.route,
            amount: amount.0,
        };
        self.pending_living_payment_by_token
            .insert(&request.token_id, &pending);

        events::log_event(
            "living_payment_pending",
            serde_json::json!({
                "token_id": request.token_id,
                "settlement_id": settlement_id,
                "payer_id": pending.payer_id,
                "action": request.action,
                "route": request.route,
                "amount": amount,
            }),
        );

        let route_promise = match request.route {
            EconomicRoute::Sponsor => ext_nuve_ft::ext(self.nuve_ft_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(GAS_FOR_NUVE_ROUTE)
                .ft_transfer(
                    self.treasury_id.clone(),
                    amount,
                    Some(format!("NUVI Living action settlement #{}", settlement_id)),
                ),
            EconomicRoute::Burn => ext_nuve_ft::ext(self.nuve_ft_id.clone())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .with_static_gas(GAS_FOR_NUVE_ROUTE)
                .burn(amount),
        };

        PromiseOrValue::Promise(
            route_promise.then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_LIVING_PAYMENT_CALLBACK)
                    .on_living_payment_routed(request.token_id, settlement_id, amount),
            ),
        )
    }
}

impl Contract {
    fn new_clean(
        config: ContractConfigInput,
        nuve_ft_id: AccountId,
        treasury_id: AccountId,
    ) -> Self {
        Self::validate_contract_metadata(&config.metadata);
        let owner_id = config.owner_id.clone();
        let metadata = config.metadata.clone();
        let mut contract_config = ContractConfig::from_input(config);
        contract_config.min_storage_deposit = contract_config
            .min_storage_deposit
            .max(Self::computed_min_storage_deposit());
        Self::validate_config(&contract_config);
        Self {
            owner_id,
            proposed_owner_id: None,
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            tokens_by_id: LookupMap::new(StorageKey::TokensById),
            token_ids: UnorderedSet::new(StorageKey::TokenIds),
            tokens_per_owner: LookupMap::new(StorageKey::TokensPerOwner),
            token_metadata_by_id: LookupMap::new(StorageKey::TokenMetadataById),
            approvals_by_id: LookupMap::new(StorageKey::ApprovalsById),
            next_approval_id_by_id: LookupMap::new(StorageKey::NextApprovalIdById),
            royalties_by_id: LookupMap::new(StorageKey::RoyaltiesById),
            next_token_id: 1,
            admins: UnorderedSet::new(StorageKey::Admins),
            permissions_by_account: LookupMap::new(StorageKey::PermissionsByAccount),
            pause_flags: PauseFlags::default(),
            config: contract_config,
            storage_deposits: LookupMap::new(StorageKey::StorageDeposits),
            state_version: STATE_VERSION_V7,
            class_max_supply_by_id: LookupMap::new(StorageKey::ClassMaxSupplyById),
            token_class_by_id: LookupMap::new(StorageKey::TokenClassById),
            class_token_by_serial: LookupMap::new(StorageKey::ClassTokenBySerial),
            living_state_by_id: LookupMap::new(StorageKey::LivingStateById),
            nuve_ft_id,
            treasury_id,
            pending_living_payment_by_token: LookupMap::new(
                StorageKey::PendingLivingPaymentByToken,
            ),
            next_living_settlement_id: 1,
            global_treasury_total: 0,
            global_burn_total: 0,
            starter_claimed_by_account: LookupMap::new(StorageKey::StarterClaimedByAccount),
        }
    }

    fn derive_nuvi_account_id(token_id: &TokenId) -> AccountId {
        assert!(
            !token_id.is_empty() && token_id.bytes().all(|byte| byte.is_ascii_digit()),
            "Hackathon NUVI Account requires numeric token_id"
        );
        format!("nuvi-{}.{}", token_id, env::current_account_id())
            .parse()
            .expect("Derived NUVI Account ID is invalid or too long")
    }

    fn nuvi_account_child_balance_yocto() -> u128 {
        (NUVI_ACCOUNT_WASM.len() as u128)
            .checked_mul(env::storage_byte_cost().as_yoctonear())
            .and_then(|value| value.checked_add(NUVI_ACCOUNT_OPERATIONAL_RESERVE_YOCTO))
            .expect("NUVI Account balance overflow")
    }

    fn nuvi_account_required_deposit_yocto() -> u128 {
        Self::nuvi_account_child_balance_yocto()
            .checked_add(NUVE_STORAGE_DEPOSIT_YOCTO)
            .expect("NUVI Account provisioning deposit overflow")
    }

    fn store_nuvi_account_id(&mut self, token_id: &TokenId, nuvi_account_id: &AccountId) {
        self.set_nuvi_account_metadata_status(
            token_id,
            "active",
            Some(nuvi_account_id),
            None,
            true,
        );
    }

    fn set_nuvi_account_metadata_status(
        &mut self,
        token_id: &TokenId,
        status: &str,
        expected_account_id: Option<&AccountId>,
        failure_phase: Option<&str>,
        active: bool,
    ) {
        let mut metadata = self
            .token_metadata_by_id
            .get(token_id)
            .expect("Token metadata not found");
        let mut extra = metadata
            .extra
            .as_deref()
            .and_then(|raw| near_sdk::serde_json::from_str::<near_sdk::serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        extra.insert(
            "nuvi_account_status".to_string(),
            near_sdk::serde_json::Value::String(status.to_string()),
        );

        if let Some(account_id) = expected_account_id {
            extra.insert(
                "nuvi_account_expected_id".to_string(),
                near_sdk::serde_json::Value::String(account_id.to_string()),
            );
            if active {
                extra.insert(
                    "nuvi_account_id".to_string(),
                    near_sdk::serde_json::Value::String(account_id.to_string()),
                );
            }
        }

        match failure_phase {
            Some(phase) => {
                extra.insert(
                    "nuvi_account_failure_phase".to_string(),
                    near_sdk::serde_json::Value::String(phase.to_string()),
                );
            }
            None => {
                extra.remove("nuvi_account_failure_phase");
            }
        }

        metadata.extra = Some(near_sdk::serde_json::Value::Object(extra).to_string());
        self.validate_metadata(&metadata);
        self.token_metadata_by_id.insert(token_id, &metadata);
    }

    fn validate_contract_metadata(metadata: &NFTContractMetadata) {
        assert!(!metadata.spec.is_empty(), "Metadata spec is required");
        assert!(!metadata.name.is_empty(), "Metadata name is required");
        assert!(!metadata.symbol.is_empty(), "Metadata symbol is required");
    }

    fn validate_config(config: &ContractConfig) {
        assert!(config.max_batch > 0, "max_batch must be positive");
        assert!(config.max_batch <= 100, "max_batch exceeds hard cap");
        assert!(
            config.max_tokens_per_view > 0,
            "max_tokens_per_view must be positive"
        );
        assert!(
            config.max_tokens_per_view <= 1_000,
            "max_tokens_per_view exceeds hard cap"
        );
        assert!(
            config.max_approvals_per_token > 0,
            "max_approvals_per_token must be positive"
        );
        assert!(
            config.max_approvals_per_token <= 100,
            "max_approvals_per_token exceeds hard cap"
        );
        assert!(
            config.max_royalty_bps < BPS_DENOMINATOR,
            "max_royalty_bps must be below 10000"
        );
        assert!(config.max_payout_len > 0, "max_payout_len must be positive");
        assert!(
            config.max_payout_len <= 50,
            "max_payout_len exceeds hard cap"
        );
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Caller is not owner"
        );
    }

    fn assert_one_yocto(&self) {
        assert_eq!(
            env::attached_deposit().as_yoctonear(),
            1,
            "Requires attached deposit of exactly 1 yoctoNEAR"
        );
    }

    fn is_owner_or_admin(&self, account_id: &AccountId) -> bool {
        *account_id == self.owner_id || self.admins.contains(account_id)
    }

    fn account_has(&self, account_id: &AccountId, permission: Permission) -> bool {
        self.permissions_by_account
            .get(account_id)
            .unwrap_or_default()
            .contains(&permission)
    }

    fn assert_can_mint(&self) {
        assert!(!self.pause_flags.mint_paused, "Mint is paused");
        let caller = env::predecessor_account_id();
        assert!(
            self.is_owner_or_admin(&caller) || self.account_has(&caller, Permission::Mint),
            "Caller is not authorized to mint"
        );
    }

    fn assert_can_burn(&self, token: &Token) {
        let caller = env::predecessor_account_id();
        assert!(
            token.owner_id == caller
                || self.is_owner_or_admin(&caller)
                || self.account_has(&caller, Permission::Burn),
            "Caller is not authorized to burn"
        );
    }

    fn assert_approved_or_owner(&self, token: &Token, approval_id: Option<ApprovalId>) {
        let caller = env::predecessor_account_id();
        if caller == token.owner_id {
            return;
        }
        let actual = token
            .approved_account_ids
            .get(&caller)
            .expect("Unauthorized");
        if let Some(expected) = approval_id {
            assert_eq!(*actual, expected, "Approval ID mismatch");
        }
    }

    fn assert_token_id_available(&self, token_id: &TokenId) {
        assert!(
            self.tokens_by_id.get(token_id).is_none(),
            "Token already exists"
        );
    }

    fn assert_no_pending_living_payment(&self, token_id: &TokenId) {
        assert!(
            self.pending_living_payment_by_token.get(token_id).is_none(),
            "Living payment settlement is pending"
        );
    }

    fn assert_living_token_unlocked(&self, token_id: &TokenId) {
        self.assert_no_pending_living_payment(token_id);
        if let Some(state) = self.living_state_by_id.get(token_id) {
            assert!(
                state.active_action.is_none(),
                "NFT is locked while a Living action is active"
            );
        }
    }

    fn generate_next_token_id(&mut self) -> TokenId {
        let token_id = self.next_token_id.to_string();
        self.assert_token_id_available(&token_id);
        self.next_token_id = self
            .next_token_id
            .checked_add(1)
            .expect("next_token_id overflow");
        token_id
    }

    fn internal_mint(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        metadata: TokenMetadata,
        royalty: Option<RoyaltyConfig>,
        memo: Option<String>,
        class: Option<TokenClass>,
    ) -> TokenView {
        self.validate_mint_input(&metadata, royalty.as_ref(), class.as_ref());
        let token = Token {
            token_id: token_id.clone(),
            owner_id: receiver_id.clone(),
            approved_account_ids: HashMap::new(),
        };
        self.token_ids.insert(&token_id);
        self.tokens_by_id.insert(&token_id, &token);
        self.token_metadata_by_id.insert(&token_id, &metadata);
        if let Some(royalty) = royalty {
            self.royalties_by_id.insert(&token_id, &royalty);
        }
        if let Some(class) = class {
            self.token_class_by_id.insert(&token_id, &class);
            self.class_token_by_serial
                .insert(&(class.class_id, class.serial), &token_id);
        }
        self.living_state_by_id
            .insert(&token_id, &LivingState::default());
        self.internal_add_token_to_owner(&receiver_id, &token_id);
        events::log_nft_mint(&receiver_id, vec![token_id], memo);
        self.token_view(token)
    }

    fn internal_burn(&mut self, token_id: &TokenId, memo: Option<String>) {
        let token = self.tokens_by_id.remove(token_id).expect("Token not found");
        self.token_ids.remove(token_id);
        self.token_metadata_by_id.remove(token_id);
        self.approvals_by_id.remove(token_id);
        self.next_approval_id_by_id.remove(token_id);
        self.royalties_by_id.remove(token_id);
        self.token_class_by_id.remove(token_id);
        self.living_state_by_id.remove(token_id);
        self.internal_remove_token_from_owner(&token.owner_id, token_id);
        events::log_nft_burn(&token.owner_id, vec![token_id.clone()], memo);
    }

    fn internal_transfer_unchecked(
        &mut self,
        token_id: &TokenId,
        new_owner_id: &AccountId,
        memo: Option<String>,
    ) -> AccountId {
        let mut token = self.tokens_by_id.get(token_id).expect("Token not found");
        let old_owner_id = token.owner_id.clone();
        assert_ne!(old_owner_id, *new_owner_id, "Self-transfer is not allowed");
        self.internal_remove_token_from_owner(&old_owner_id, token_id);
        self.internal_add_token_to_owner(new_owner_id, token_id);
        token.owner_id = new_owner_id.clone();
        token.approved_account_ids.clear();
        self.approvals_by_id.remove(token_id);
        self.tokens_by_id.insert(token_id, &token);
        events::log_nft_transfer(&old_owner_id, new_owner_id, vec![token_id.clone()], memo);
        old_owner_id
    }

    fn token_view(&self, token: Token) -> TokenView {
        let stored_metadata = self
            .token_metadata_by_id
            .get(&token.token_id)
            .expect("Token metadata not found");

        TokenView {
            token_id: token.token_id.clone(),
            owner_id: token.owner_id,
            metadata: self.dynamic_token_metadata(&token.token_id, stored_metadata),
            approved_account_ids: token.approved_account_ids,
        }
    }

    fn dynamic_token_metadata(
        &self,
        token_id: &TokenId,
        mut metadata: TokenMetadata,
    ) -> TokenMetadata {
        let Some(state) = self.living_state_by_id.get(token_id) else {
            return metadata;
        };

        // Caller-supplied explicit media remains authoritative. Starter NUVI
        // intentionally stores no media, so its public NEP-171 view derives
        // the current visual form from collection.base_uri.
        if metadata.media.is_none() {
            if let Some(collection) = self.metadata.get() {
                if let Some(base_uri) = collection.base_uri {
                    let form_file = match state.form_id.as_str() {
                        BASIC_FORM_ID => Some("basic.png"),
                        FORM_A_ID => Some("form_a.png"),
                        FORM_B_ID => Some("form_b.png"),
                        _ => None,
                    };
                    if let Some(form_file) = form_file {
                        metadata.media =
                            Some(format!("{}/{}", base_uri.trim_end_matches('/'), form_file));
                    }
                }
            }
        }

        let mut extra = metadata
            .extra
            .as_deref()
            .and_then(|raw| near_sdk::serde_json::from_str::<near_sdk::serde_json::Value>(raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        extra.insert(
            "form_id".to_string(),
            near_sdk::serde_json::Value::String(state.form_id.clone()),
        );
        extra.insert(
            "level".to_string(),
            near_sdk::serde_json::Value::from(state.level),
        );
        extra.insert(
            "xp".to_string(),
            near_sdk::serde_json::Value::from(state.xp),
        );
        extra.insert(
            "mutation_count".to_string(),
            near_sdk::serde_json::Value::from(state.mutation_count),
        );
        extra.insert(
            "work_a_xp".to_string(),
            near_sdk::serde_json::Value::from(state.work_a_xp),
        );
        extra.insert(
            "work_b_xp".to_string(),
            near_sdk::serde_json::Value::from(state.work_b_xp),
        );
        extra.insert(
            "sponsor_spent".to_string(),
            near_sdk::serde_json::Value::String(state.sponsor_spent.to_string()),
        );
        extra.insert(
            "burner_spent".to_string(),
            near_sdk::serde_json::Value::String(state.burner_spent.to_string()),
        );
        extra.insert(
            "active_action".to_string(),
            near_sdk::serde_json::to_value(state.active_action)
                .expect("Living action must serialize"),
        );
        extra.insert(
            "ready_at_ns".to_string(),
            near_sdk::serde_json::Value::from(state.ready_at_ns),
        );

        metadata.extra = Some(near_sdk::serde_json::Value::Object(extra).to_string());
        metadata
    }

    fn internal_add_token_to_owner(&mut self, owner_id: &AccountId, token_id: &TokenId) {
        let mut set = self
            .tokens_per_owner
            .get(owner_id)
            .unwrap_or_else(|| UnorderedSet::new(Self::owner_set_prefix(owner_id)));
        set.insert(token_id);
        self.tokens_per_owner.insert(owner_id, &set);
    }

    fn internal_remove_token_from_owner(&mut self, owner_id: &AccountId, token_id: &TokenId) {
        let mut set = self
            .tokens_per_owner
            .get(owner_id)
            .expect("Owner token set not found");
        set.remove(token_id);
        self.tokens_per_owner.insert(owner_id, &set);
    }

    fn owner_set_prefix(account_id: &AccountId) -> StorageKey {
        StorageKey::TokensPerOwnerInner {
            account_hash: env::sha256(account_id.as_bytes()),
        }
    }

    fn initialize_living_states_for_existing_tokens(&mut self) {
        let token_ids = self.token_ids.iter().collect::<Vec<_>>();

        for token_id in token_ids {
            if self.living_state_by_id.get(&token_id).is_none() {
                self.living_state_by_id
                    .insert(&token_id, &LivingState::default());
            }
        }
    }

    fn validate_metadata(&self, metadata: &TokenMetadata) {
        if let Some(extra) = &metadata.extra {
            assert!(
                extra.len() <= self.config.max_metadata_extra_bytes as usize,
                "Metadata extra is too large"
            );
        }
    }

    fn validate_class_id(class_id: &ClassId) {
        let bytes = class_id.as_bytes();
        assert!(!bytes.is_empty(), "class_id must not be empty");
        assert!(
            bytes.len() <= MAX_CLASS_ID_BYTES,
            "class_id exceeds maximum length"
        );
        assert!(
            bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit(),
            "Invalid class_id"
        );
        assert!(
            bytes.iter().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(*byte, b'.' | b'_' | b'-')
            }),
            "Invalid class_id"
        );
    }

    fn validate_token_class(&self, class: &TokenClass) {
        Self::validate_class_id(&class.class_id);
        let max_supply = self
            .class_max_supply_by_id
            .get(&class.class_id)
            .expect("Token class not found");
        assert!(class.serial >= 1, "serial must be at least 1");
        assert!(class.serial <= max_supply, "serial exceeds max_supply");
        assert!(
            self.class_token_by_serial
                .get(&(class.class_id.clone(), class.serial))
                .is_none(),
            "Class serial already used"
        );
    }

    fn validate_mint_input(
        &self,
        metadata: &TokenMetadata,
        royalty: Option<&RoyaltyConfig>,
        class: Option<&TokenClass>,
    ) {
        self.validate_metadata(metadata);
        if let Some(royalty) = royalty {
            self.validate_royalty(royalty);
        }
        if let Some(class) = class {
            self.validate_token_class(class);
        }
    }

    fn validate_royalty(&self, royalty: &RoyaltyConfig) {
        assert!(!royalty.payout.is_empty(), "Invalid royalty configuration");
        assert!(
            royalty.payout.len() < self.config.max_payout_len as usize,
            "Invalid royalty configuration"
        );
        let mut total = 0u32;
        for (account_id, bps) in &royalty.payout {
            assert!(
                env::is_valid_account_id(account_id.as_bytes()),
                "Invalid royalty configuration"
            );
            assert!(*bps > 0, "Invalid royalty configuration");
            total = total.checked_add(*bps).expect("Royalty overflow");
        }
        assert!(
            total <= self.config.max_royalty_bps,
            "Invalid royalty configuration"
        );
        assert!(total < BPS_DENOMINATOR, "Invalid royalty configuration");
    }

    fn calculate_payout(&self, token_id: &TokenId, balance: U128, max_len_payout: u32) -> Payout {
        assert!(
            max_len_payout <= self.config.max_payout_len,
            "max_len_payout exceeds config"
        );
        let token = self.tokens_by_id.get(token_id).expect("Token not found");
        let royalty = self.royalties_by_id.get(token_id);
        let mut payout = HashMap::new();
        let mut royalty_total = 0u128;
        if let Some(royalty) = royalty {
            for (account, bps) in royalty.payout {
                let amount = balance.0.checked_mul(bps as u128).expect("Payout overflow")
                    / BPS_DENOMINATOR as u128;
                royalty_total = royalty_total.checked_add(amount).expect("Payout overflow");
                if amount > 0 {
                    payout
                        .entry(account)
                        .and_modify(|value: &mut U128| {
                            value.0 = value.0.checked_add(amount).expect("Payout overflow");
                        })
                        .or_insert(U128(amount));
                }
            }
        }
        let seller_amount = balance.0.saturating_sub(royalty_total);
        payout
            .entry(token.owner_id)
            .and_modify(|value: &mut U128| {
                value.0 = value.0.checked_add(seller_amount).expect("Payout overflow");
            })
            .or_insert(U128(seller_amount));
        assert!(
            payout.len() <= max_len_payout as usize,
            "Payout length exceeds max_len_payout"
        );
        Payout { payout }
    }

    fn checked_view_limit(&self, limit: Option<u64>) -> u64 {
        let limit = limit.unwrap_or(self.config.max_tokens_per_view);
        assert!(
            limit <= self.config.max_tokens_per_view,
            "View limit exceeds max_tokens_per_view"
        );
        limit
    }

    fn paginate_set<T>(
        &self,
        set: &UnorderedSet<T>,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<T>
    where
        T: BorshSerialize + BorshDeserialize,
    {
        let limit = self.checked_view_limit(limit);
        set.iter()
            .skip(from_index.unwrap_or(U128(0)).0 as usize)
            .take(limit as usize)
            .collect()
    }

    fn computed_min_storage_deposit() -> u128 {
        env::storage_byte_cost().as_yoctonear() * MIN_STORAGE_BYTES
    }

    fn minimum_storage_deposit(&self) -> u128 {
        self.config
            .min_storage_deposit
            .max(Self::computed_min_storage_deposit())
    }

    fn refund_deposit(&self, account_id: &AccountId, amount: u128) {
        if amount > 0 {
            Promise::new(account_id.clone())
                .transfer(NearToken::from_yoctonear(amount))
                .detach();
        }
    }

    fn refund_storage_deposit(&self, account_id: &AccountId, initial_storage: u64) {
        let attached = env::attached_deposit().as_yoctonear();
        let used = env::storage_usage().saturating_sub(initial_storage) as u128;
        let required = used * env::storage_byte_cost().as_yoctonear();
        assert!(attached >= required, "Insufficient attached deposit");
        self.refund_deposit(account_id, attached - required);
    }

    fn refund_released_storage(&self, account_id: &AccountId, initial_storage: u64) {
        let released = initial_storage.saturating_sub(env::storage_usage()) as u128;
        let refund = released * env::storage_byte_cost().as_yoctonear();
        self.refund_deposit(account_id, refund);
    }

    fn settle_storage_cost(
        &mut self,
        account_id: &AccountId,
        initial_storage: u64,
        allow_registered_balance: bool,
    ) {
        let attached = env::attached_deposit().as_yoctonear();
        let used = env::storage_usage().saturating_sub(initial_storage) as u128;
        let required = used * env::storage_byte_cost().as_yoctonear();
        if attached >= required {
            self.refund_deposit(account_id, attached - required);
            return;
        }
        assert!(allow_registered_balance, "Insufficient attached deposit");
        let remainder = required - attached;
        let current = self.storage_deposits.get(account_id).unwrap_or(0);
        assert!(current >= remainder, "Insufficient storage balance");
        self.storage_deposits
            .insert(account_id, &(current - remainder));
    }
}

#[cfg(test)]
mod tests;
