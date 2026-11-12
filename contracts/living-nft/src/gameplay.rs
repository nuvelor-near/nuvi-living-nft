use crate::{LivingActionKind, LivingState, TokenId};
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

pub const MUTATION_THRESHOLD_XP: u64 = 5;
pub const BASIC_FORM_ID: &str = "basic";
pub const FORM_A_ID: &str = "form_a";
pub const FORM_B_ID: &str = "form_b";

const MUTATION_BASE_WEIGHT: u64 = 10;
const MUTATION_WORK_XP_WEIGHT: u64 = 20;
const MUTATION_BEHAVIOR_LEVEL_WEIGHT: u64 = 5;

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
    pub mutation_threshold_xp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct GlobalAlignmentView {
    pub treasury_total: U128,
    pub burn_total: U128,
    pub global_total: U128,
    pub treasury_share_bps: u32,
    pub burn_share_bps: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct MutationPreviewView {
    pub token_id: TokenId,
    pub eligible: bool,
    pub current_form_id: String,
    pub mutation_stage: u32,
    pub xp: u64,
    pub threshold_xp: u64,
    pub work_a_xp: u64,
    pub work_b_xp: u64,
    pub sponsor_level: u8,
    pub burner_level: u8,
    pub form_a_weight: u64,
    pub form_b_weight: u64,
    pub form_a_probability_bps: u32,
    pub form_b_probability_bps: u32,
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

pub fn behavior_level(spent_atomic: u128) -> u8 {
    if spent_atomic >= 40 * NUVE_UNIT {
        3
    } else if spent_atomic >= 20 * NUVE_UNIT {
        2
    } else if spent_atomic >= 5 * NUVE_UNIT {
        1
    } else {
        0
    }
}

pub fn mutation_weights(state: &LivingState) -> (u64, u64, u8, u8) {
    let sponsor_level = behavior_level(state.sponsor_spent);
    let burner_level = behavior_level(state.burner_spent);

    let form_a_weight = MUTATION_BASE_WEIGHT
        .saturating_add(state.work_a_xp.saturating_mul(MUTATION_WORK_XP_WEIGHT))
        .saturating_add((sponsor_level as u64).saturating_mul(MUTATION_BEHAVIOR_LEVEL_WEIGHT));

    let form_b_weight = MUTATION_BASE_WEIGHT
        .saturating_add(state.work_b_xp.saturating_mul(MUTATION_WORK_XP_WEIGHT))
        .saturating_add((burner_level as u64).saturating_mul(MUTATION_BEHAVIOR_LEVEL_WEIGHT));

    (form_a_weight, form_b_weight, sponsor_level, burner_level)
}

pub fn mutation_probability_bps(form_a_weight: u64, form_b_weight: u64) -> (u32, u32) {
    let total = form_a_weight.saturating_add(form_b_weight);
    assert!(total > 0, "Mutation weight total must be positive");

    let form_a_bps = ((form_a_weight as u128) * 10_000u128 / (total as u128)) as u32;
    (form_a_bps, 10_000u32 - form_a_bps)
}

pub fn global_alignment_bps(treasury_total: u128, burn_total: u128) -> (u32, u32) {
    let global_total = treasury_total.saturating_add(burn_total);
    if global_total == 0 {
        return (0, 0);
    }

    let treasury_share_bps = (treasury_total.saturating_mul(10_000u128) / global_total) as u32;
    (treasury_share_bps, 10_000u32 - treasury_share_bps)
}
