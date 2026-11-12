use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};

pub const DEFAULT_FORM_ID: &str = "basic";

#[repr(u8)]
#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq,
)]
#[borsh(use_discriminant = true)]
#[serde(crate = "near_sdk::serde", rename_all = "snake_case")]
pub enum LivingActionKind {
    Feed = 0,
    WorkA = 1,
    WorkB = 2,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, PartialEq, Eq)]
pub struct LivingState {
    /// Current visual/evolution form. Mutation changes this value
    /// without changing token_id.
    pub form_id: String,

    /// Persistent character progression.
    pub level: u32,
    pub xp: u64,

    /// Optional dominant specialization selected/derived later.
    pub specialization: Option<String>,

    /// Number of completed mutations for this NFT.
    pub mutation_count: u32,

    /// Personal work affinities used by mutation weighting.
    pub work_a_xp: u64,
    pub work_b_xp: u64,

    /// Raw atomic NUVE amounts routed through each economic choice.
    pub sponsor_spent: u128,
    pub burner_spent: u128,

    /// Active timed gameplay action, if any.
    pub active_action: Option<LivingActionKind>,

    /// NEAR block timestamp in nanoseconds when active_action becomes ready.
    /// Zero means there is no active timer.
    pub ready_at_ns: u64,
}

impl Default for LivingState {
    fn default() -> Self {
        Self {
            form_id: DEFAULT_FORM_ID.to_string(),
            level: 1,
            xp: 0,
            specialization: None,
            mutation_count: 0,
            work_a_xp: 0,
            work_b_xp: 0,
            sponsor_spent: 0,
            burner_spent: 0,
            active_action: None,
            ready_at_ns: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct LivingStateView {
    pub form_id: String,
    pub level: u32,
    pub xp: u64,
    pub specialization: Option<String>,
    pub mutation_count: u32,
    pub work_a_xp: u64,
    pub work_b_xp: u64,

    /// JSON string representation via U128.
    pub sponsor_spent: U128,
    pub burner_spent: U128,

    pub active_action: Option<LivingActionKind>,
    pub ready_at_ns: u64,
}

impl From<LivingState> for LivingStateView {
    fn from(state: LivingState) -> Self {
        Self {
            form_id: state.form_id,
            level: state.level,
            xp: state.xp,
            specialization: state.specialization,
            mutation_count: state.mutation_count,
            work_a_xp: state.work_a_xp,
            work_b_xp: state.work_b_xp,
            sponsor_spent: U128(state.sponsor_spent),
            burner_spent: U128(state.burner_spent),
            active_action: state.active_action,
            ready_at_ns: state.ready_at_ns,
        }
    }
}
