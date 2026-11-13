use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near, near_bindgen, AccountId, Gas, NearToken, PanicOnDefault, Promise,
};

const GAS_FOR_OWNER_LOOKUP: Gas = Gas::from_tgas(25);
const GAS_FOR_OWNER_CALLBACK: Gas = Gas::from_tgas(25);
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(25);
const MAX_OWNER_RESULT_BYTES: usize = 16 * 1024;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    nft_contract_id: AccountId,
    token_id: String,
    nuve_ft_id: AccountId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct ConfigView {
    pub nft_contract_id: AccountId,
    pub token_id: String,
    pub nuve_ft_id: AccountId,
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct NftTokenOwnerView {
    owner_id: AccountId,
}

#[ext_contract(ext_nft)]
trait BoundNft {
    fn nft_token(&self, token_id: String) -> Option<NftTokenOwnerView>;
}

#[ext_contract(ext_nuve)]
trait NuveFt {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_self)]
trait Callbacks {
    fn on_owner_checked(
        &self,
        requester_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> Promise;
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(nft_contract_id: AccountId, token_id: String, nuve_ft_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Contract is already initialized");
        assert!(!token_id.trim().is_empty(), "token_id must not be empty");
        Self {
            nft_contract_id,
            token_id,
            nuve_ft_id,
        }
    }

    pub fn get_config(&self) -> ConfigView {
        ConfigView {
            nft_contract_id: self.nft_contract_id.clone(),
            token_id: self.token_id.clone(),
            nuve_ft_id: self.nuve_ft_id.clone(),
        }
    }

    pub fn request_ft_transfer(&self, receiver_id: AccountId, amount: U128) -> Promise {
        assert!(amount.0 > 0, "amount must be positive");
        let requester_id = env::predecessor_account_id();

        ext_nft::ext(self.nft_contract_id.clone())
            .with_static_gas(GAS_FOR_OWNER_LOOKUP)
            .nft_token(self.token_id.clone())
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_OWNER_CALLBACK)
                    .on_owner_checked(requester_id, receiver_id, amount),
            )
    }

    #[private]
    pub fn on_owner_checked(
        &self,
        requester_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> Promise {
        assert!(amount.0 > 0, "amount must be positive");

        let raw = env::promise_result_checked(0, MAX_OWNER_RESULT_BYTES)
            .unwrap_or_else(|_| env::panic_str("NFT owner lookup failed"));
        let token: Option<NftTokenOwnerView> = near_sdk::serde_json::from_slice(&raw)
            .unwrap_or_else(|_| env::panic_str("Invalid nft_token response"));
        let token = token.unwrap_or_else(|| env::panic_str("Bound NFT does not exist"));

        assert_eq!(
            token.owner_id, requester_id,
            "Caller is not current owner of bound NUVI NFT"
        );

        ext_nuve::ext(self.nuve_ft_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .ft_transfer(
                receiver_id,
                amount,
                Some(format!("NUVI Account transfer for token {}", self.token_id)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, PromiseResult};
    use std::collections::HashMap;

    fn account(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn context(predecessor: &str) {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(account("nuvi-1.nuvi-hack26.testnet"))
            .predecessor_account_id(account(predecessor));
        testing_env!(builder.build());
    }

    fn callback_context(owner_id: &str) {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(account("nuvi-1.nuvi-hack26.testnet"))
            .predecessor_account_id(account("nuvi-1.nuvi-hack26.testnet"));
        let bytes = near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
            "owner_id": owner_id,
            "token_id": "1",
            "metadata": {},
            "approved_account_ids": {}
        }))
        .unwrap();
        testing_env!(
            builder.build(),
            near_sdk::test_vm_config(),
            near_sdk::RuntimeFeesConfig::test(),
            HashMap::new(),
            vec![PromiseResult::Successful(bytes)],
        );
    }

    fn contract() -> Contract {
        context("creator.testnet");
        Contract::new(
            account("nuvi-hack26.testnet"),
            "1".to_string(),
            account("nuve-hack26.testnet"),
        )
    }

    #[test]
    fn init_and_config_are_immutable_binding() {
        let contract = contract();
        assert_eq!(
            contract.get_config(),
            ConfigView {
                nft_contract_id: account("nuvi-hack26.testnet"),
                token_id: "1".to_string(),
                nuve_ft_id: account("nuve-hack26.testnet"),
            }
        );
    }

    #[test]
    #[should_panic(expected = "token_id must not be empty")]
    fn empty_token_id_is_rejected() {
        context("creator.testnet");
        Contract::new(
            account("nuvi-hack26.testnet"),
            " ".to_string(),
            account("nuve-hack26.testnet"),
        );
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn zero_amount_is_rejected_before_owner_lookup() {
        let contract = contract();
        context("alice.testnet");
        let _ = contract.request_ft_transfer(account("bob.testnet"), U128(0));
    }

    #[test]
    fn current_nft_owner_can_reach_ft_transfer_promise() {
        let contract = contract();
        callback_context("alice.testnet");
        let _ = contract.on_owner_checked(
            account("alice.testnet"),
            account("bob.testnet"),
            U128(10_000),
        );
    }

    #[test]
    #[should_panic(expected = "Caller is not current owner of bound NUVI NFT")]
    fn stale_or_non_owner_is_rejected() {
        let contract = contract();
        callback_context("bob.testnet");
        let _ = contract.on_owner_checked(
            account("alice.testnet"),
            account("alice.testnet"),
            U128(10_000),
        );
    }
}
