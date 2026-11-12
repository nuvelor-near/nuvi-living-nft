use borsh::{BorshDeserialize, BorshSerialize};
use near_contract_standards::non_fungible_token::metadata::NFTContractMetadata;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use std::collections::HashMap;

pub type TokenId = String;
pub type ClassId = String;
pub type ApprovalId = u64;
pub type PermissionSet = Vec<Permission>;

#[repr(u8)]
#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
)]
#[borsh(use_discriminant = true)]
#[serde(crate = "near_sdk::serde")]
pub enum Permission {
    Mint = 0,
    Burn = 1,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct RoyaltyConfig {
    pub payout: HashMap<AccountId, u32>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct PauseFlags {
    pub transfers_paused: bool,
    pub mint_paused: bool,
    pub burn_paused: bool,
    pub approvals_paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde", deny_unknown_fields)]
pub struct ContractConfigInput {
    pub owner_id: AccountId,
    pub metadata: NFTContractMetadata,
    pub max_batch: Option<u32>,
    pub max_tokens_per_view: Option<u64>,
    pub max_approvals_per_token: Option<u32>,
    pub max_royalty_bps: Option<u32>,
    pub max_payout_len: Option<u32>,
    pub max_metadata_extra_bytes: Option<u32>,
    pub min_storage_deposit: Option<u128>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractConfig {
    pub max_batch: u32,
    pub max_tokens_per_view: u64,
    pub max_approvals_per_token: u32,
    pub max_royalty_bps: u32,
    pub max_payout_len: u32,
    pub max_metadata_extra_bytes: u32,
    pub min_storage_deposit: u128,
}

impl ContractConfig {
    pub fn from_input(input: ContractConfigInput) -> Self {
        Self {
            max_batch: input.max_batch.unwrap_or(100),
            max_tokens_per_view: input.max_tokens_per_view.unwrap_or(200),
            max_approvals_per_token: input.max_approvals_per_token.unwrap_or(10),
            max_royalty_bps: input.max_royalty_bps.unwrap_or(2_000),
            max_payout_len: input.max_payout_len.unwrap_or(10),
            max_metadata_extra_bytes: input.max_metadata_extra_bytes.unwrap_or(1_024),
            min_storage_deposit: input.min_storage_deposit.unwrap_or(0),
        }
    }

    pub fn apply_patch(&mut self, patch: ContractConfigPatch, computed_min_storage_deposit: u128) {
        if let Some(value) = patch.max_batch {
            self.max_batch = value;
        }
        if let Some(value) = patch.max_tokens_per_view {
            self.max_tokens_per_view = value;
        }
        if let Some(value) = patch.max_approvals_per_token {
            self.max_approvals_per_token = value;
        }
        if let Some(value) = patch.max_royalty_bps {
            self.max_royalty_bps = value;
        }
        if let Some(value) = patch.max_payout_len {
            self.max_payout_len = value;
        }
        if let Some(value) = patch.max_metadata_extra_bytes {
            self.max_metadata_extra_bytes = value;
        }
        if let Some(value) = patch.min_storage_deposit {
            assert!(
                value >= computed_min_storage_deposit,
                "min_storage_deposit is below computed minimum"
            );
            self.min_storage_deposit = value;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(crate = "near_sdk::serde", deny_unknown_fields)]
pub struct ContractConfigPatch {
    pub max_batch: Option<u32>,
    pub max_tokens_per_view: Option<u64>,
    pub max_approvals_per_token: Option<u32>,
    pub max_royalty_bps: Option<u32>,
    pub max_payout_len: Option<u32>,
    pub max_metadata_extra_bytes: Option<u32>,
    pub min_storage_deposit: Option<u128>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub media: Option<String>,
    pub media_hash: Option<Base64VecU8>,
    pub copies: Option<u64>,
    pub issued_at: Option<u64>,
    pub expires_at: Option<u64>,
    pub starts_at: Option<u64>,
    pub updated_at: Option<u64>,
    pub extra: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<Base64VecU8>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenClass {
    pub class_id: ClassId,
    pub serial: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde", deny_unknown_fields)]
pub struct MintTokenInput {
    pub metadata: TokenMetadata,
    pub royalty: Option<RoyaltyConfig>,
    #[serde(default)]
    pub class: Option<TokenClass>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenView {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: TokenMetadata,
    pub approved_account_ids: HashMap<AccountId, ApprovalId>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub approved_account_ids: HashMap<AccountId, ApprovalId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: HashMap<AccountId, U128>,
}
