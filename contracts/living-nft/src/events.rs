use crate::types::*;
use near_sdk::serde::Serialize;
use near_sdk::{env, AccountId};
use serde_json::json;

pub const NUVELOR_EVENT_STANDARD: &str = "nuvelor-nft";
pub const NUVELOR_EVENT_VERSION: &str = "1.0.0";

pub fn log_event<T: Serialize>(event: &str, data: T) {
    env::log_str(&format!(
        "EVENT_JSON:{}",
        json!({
            "standard": NUVELOR_EVENT_STANDARD,
            "version": NUVELOR_EVENT_VERSION,
            "event": event,
            "data": [data],
        })
    ));
}

pub fn log_nep171_event<T: Serialize>(event: &str, data: T) {
    env::log_str(&format!(
        "EVENT_JSON:{}",
        json!({
            "standard": "nep171",
            "version": "1.0.0",
            "event": event,
            "data": [data],
        })
    ));
}

pub fn log_nft_mint(owner_id: &AccountId, token_ids: Vec<TokenId>, memo: Option<String>) {
    log_nep171_event(
        "nft_mint",
        json!({
            "owner_id": owner_id,
            "token_ids": token_ids,
            "memo": memo,
        }),
    );
}

pub fn log_nft_transfer(
    old_owner_id: &AccountId,
    new_owner_id: &AccountId,
    token_ids: Vec<TokenId>,
    memo: Option<String>,
) {
    log_nep171_event(
        "nft_transfer",
        json!({
            "old_owner_id": old_owner_id,
            "new_owner_id": new_owner_id,
            "token_ids": token_ids,
            "memo": memo,
        }),
    );
}

pub fn log_nft_burn(owner_id: &AccountId, token_ids: Vec<TokenId>, memo: Option<String>) {
    log_nep171_event(
        "nft_burn",
        json!({
            "owner_id": owner_id,
            "token_ids": token_ids,
            "memo": memo,
        }),
    );
}
