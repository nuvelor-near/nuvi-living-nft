use crate::{LivingActionKind, TokenId};
use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;

pub const NUVE_DECIMALS: u8 = 4;
pub const NUVE_UNIT: u128 = 10_000;

pub const FEED_COST_ATOMIC: u128 = 5 * NUVE_UNIT;
pub const WORK_COST_ATOMIC: u128 = 10 * NUVE_UNIT;

pub const FEED_DURATION_NS: u64 = 30_000_000_000;
pub const WORK_DURATION_NS: u64 = 10_000_000_000;

#[repr(u8)]
#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq,
)]
#[borsh(use_discriminant = true)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
pub enum EconomicRoute {
    Sponsor = 0,
    Burn = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde", deny_unknown_fields)]
pub struct LivingPaymentMessage {
    pub token_id: TokenId,
    pub action: LivingActionKind,
    pub route: EconomicRoute,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq, Eq)]
pub struct PendingLivingPayment {
    pub settlement_id: u64,
    pub token_id: TokenId,
    pub payer_id: AccountId,
    pub action: LivingActionKind,
    pub route: EconomicRoute,
    pub amount: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct PendingLivingPaymentView {
    pub settlement_id: u64,
    pub token_id: TokenId,
    pub payer_id: AccountId,
    pub action: LivingActionKind,
    pub route: EconomicRoute,
    pub amount: U128,
}

impl From<PendingLivingPayment> for PendingLivingPaymentView {
    fn from(value: PendingLivingPayment) -> Self {
        Self {
            settlement_id: value.settlement_id,
            token_id: value.token_id,
            payer_id: value.payer_id,
            action: value.action,
            route: value.route,
            amount: U128(value.amount),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct LivingGameConfigView {
    pub nuve_ft_id: AccountId,
    pub treasury_id: AccountId,
    pub nuve_decimals: u8,
    pub feed_cost: U128,
    pub work_cost: U128,
    pub feed_duration_ns: u64,
    pub work_duration_ns: u64,
}

pub fn action_cost_atomic(action: LivingActionKind) -> u128 {
    match action {
        LivingActionKind::Feed => FEED_COST_ATOMIC,
        LivingActionKind::WorkA | LivingActionKind::WorkB => WORK_COST_ATOMIC,
    }
}

pub fn action_duration_ns(action: LivingActionKind) -> u64 {
    match action {
        LivingActionKind::Feed => FEED_DURATION_NS,
        LivingActionKind::WorkA | LivingActionKind::WorkB => WORK_DURATION_NS,
    }
}
