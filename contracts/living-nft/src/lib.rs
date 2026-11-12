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
const MAX_CLASS_ID_BYTES: usize = 64;
const BPS_DENOMINATOR: u32 = 10_000;
const GAS_FOR_NFT_ON_TRANSFER: Gas = Gas::from_tgas(30);
const GAS_FOR_NFT_RESOLVE_TRANSFER: Gas = Gas::from_tgas(10);
const GAS_FOR_NFT_ON_APPROVE: Gas = Gas::from_tgas(10);
const GAS_FOR_NUVE_ROUTE: Gas = Gas::from_tgas(20);
const GAS_FOR_LIVING_PAYMENT_CALLBACK: Gas = Gas::from_tgas(20);
const MIN_STORAGE_BYTES: u128 = 1_000;

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
}

#[ext_contract(ext_nuve_ft)]
#[allow(dead_code)]
trait NuveFt {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn burn(&mut self, amount: U128) -> U128;
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

#[near(
    contract_state,
    contract_metadata(
        version = "5.0.0",
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
    pub fn migrate_v2_to_v5(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
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
            state_version: STATE_VERSION_V5,
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
        };

        contract.initialize_living_states_for_existing_tokens();
        contract
    }

    #[init(ignore_state)]
    pub fn migrate_v3_to_v5(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
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
            state_version: STATE_VERSION_V5,
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
        };

        contract.initialize_living_states_for_existing_tokens();
        contract
    }

    #[init(ignore_state)]
    pub fn migrate_v4_to_v5(nuve_ft_id: AccountId, treasury_id: AccountId) -> Self {
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
            state_version: STATE_VERSION_V5,
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

        state.active_action = Some(pending.action);
        state.ready_at_ns =
            env::block_timestamp().saturating_add(action_duration_ns(pending.action));

        self.living_state_by_id.insert(&token_id, &state);
        self.pending_living_payment_by_token.remove(&token_id);

        events::log_event(
            "living_action_started",
            serde_json::json!({
                "token_id": token_id,
                "settlement_id": settlement_id,
                "payer_id": pending.payer_id,
                "action": pending.action,
                "route": pending.route,
                "amount": amount,
                "ready_at_ns": state.ready_at_ns,
            }),
        );

        U128(0)
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
        self.assert_no_pending_living_payment(&token_id);
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
            self.assert_no_pending_living_payment(token_id);
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
            state_version: STATE_VERSION_V5,
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
        }
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
        TokenView {
            token_id: token.token_id.clone(),
            owner_id: token.owner_id,
            metadata: self
                .token_metadata_by_id
                .get(&token.token_id)
                .expect("Token metadata not found"),
            approved_account_ids: token.approved_account_ids,
        }
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
