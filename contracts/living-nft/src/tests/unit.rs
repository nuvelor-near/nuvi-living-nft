use crate::*;
use near_contract_standards::non_fungible_token::metadata::NFTContractMetadata;
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::test_utils::{get_logs, VMContextBuilder};
use near_sdk::{env, testing_env, AccountId, NearToken};
use std::collections::HashMap;

fn account(name: &str) -> AccountId {
    name.parse().unwrap()
}

fn context(predecessor: &str, deposit: u128) {
    context_at(predecessor, deposit, 0);
}

fn context_at(predecessor: &str, deposit: u128, block_timestamp_ns: u64) {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(account("nft.nuvelor.near"))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .block_timestamp(block_timestamp_ns);
    testing_env!(builder.build());
}

fn nuve_ft_id() -> AccountId {
    account("nuve-demo.testnet")
}

fn treasury_id() -> AccountId {
    account("treasury.testnet")
}

fn metadata(title: &str) -> TokenMetadata {
    TokenMetadata {
        title: Some(title.to_string()),
        description: None,
        media: None,
        media_hash: None,
        copies: Some(1),
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    }
}

fn config(owner: &str) -> ContractConfigInput {
    ContractConfigInput {
        owner_id: account(owner),
        metadata: NFTContractMetadata {
            spec: "nft-1.0.0".to_string(),
            name: "NUVELOR NFT".to_string(),
            symbol: "NVLNFT".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        },
        max_batch: Some(100),
        max_tokens_per_view: Some(200),
        max_approvals_per_token: Some(10),
        max_royalty_bps: Some(2_000),
        max_payout_len: Some(10),
        max_metadata_extra_bytes: Some(1_024),
        min_storage_deposit: None,
    }
}

fn contract_metadata_with_assets() -> NFTContractMetadata {
    NFTContractMetadata {
        spec: "nft-1.0.0".to_string(),
        name: "NUVELOR Genesis".to_string(),
        symbol: "NVLG".to_string(),
        icon: Some("ipfs://bafybeigdyrztl7icon".to_string()),
        base_uri: Some("ipfs://bafybeigdyrztl7base".to_string()),
        reference: Some("ipfs://bafybeigdyrztl7ref/contract.json".to_string()),
        reference_hash: Some(Base64VecU8::from(vec![1, 2, 3, 4])),
    }
}

fn contract() -> Contract {
    context("owner.near", 0);
    Contract::new(config("owner.near"), nuve_ft_id(), treasury_id())
}

fn mint(contract: &mut Contract, title: &str, owner: &str) -> TokenView {
    context("owner.near", 10u128.pow(24));
    contract.nft_mint(account(owner), metadata(title), None, None, None)
}

fn repaired_token_metadata() -> TokenMetadata {
    TokenMetadata {
        title: Some("Nuvi Genesis".to_string()),
        description: Some("Genesis mascot with corrected metadata.".to_string()),
        media: Some(
            "ipfs://bafybeicz63lmkptgvkbf4oomq53qlxuyngcuelwp6rvhveuejdz6shfx24".to_string(),
        ),
        media_hash: None,
        copies: Some(1),
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: Some(
            "{\"type\":\"Mascot\",\"character\":\"Nuvi\",\"rarity\":\"Common\"}".to_string(),
        ),
        reference: Some(
            "ipfs://bafybeicz63lmkptgvkbf4oomq53qlxuyngcuelwp6rvhveuejdz6shfx24/metadata.json"
                .to_string(),
        ),
        reference_hash: Some(Base64VecU8::from(vec![9, 8, 7, 6])),
    }
}

fn living_payment_msg(token_id: TokenId, action: LivingActionKind, route: EconomicRoute) -> String {
    near_sdk::serde_json::to_string(&LivingPaymentMessage {
        token_id,
        action,
        route,
    })
    .unwrap()
}

fn callback_context(block_timestamp_ns: u64, promise_result: near_sdk::PromiseResult) {
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(account("nft.nuvelor.near"))
        .predecessor_account_id(account("nft.nuvelor.near"))
        .block_timestamp(block_timestamp_ns);
    testing_env!(
        builder.build(),
        near_sdk::test_vm_config(),
        near_sdk::RuntimeFeesConfig::test(),
        HashMap::new(),
        vec![promise_result],
    );
}

#[test]
fn init_success() {
    let contract = contract();
    assert_eq!(contract.owner_id, account("owner.near"));
    assert_eq!(contract.state_version, STATE_VERSION_V7);
    assert_eq!(contract.next_token_id, 1);
    assert_eq!(contract.config.max_batch, 100);
    assert_eq!(contract.config.max_tokens_per_view, 200);
    assert_eq!(contract.config.max_approvals_per_token, 10);
    assert_eq!(
        contract.get_global_alignment(),
        GlobalAlignmentView {
            treasury_total: U128(0),
            burn_total: U128(0),
            global_total: U128(0),
            treasury_share_bps: 0,
            burn_share_bps: 0,
        }
    );
    assert!(!contract.starter_claim_used(account("alice.near")));
}

#[test]
#[should_panic(expected = "Metadata name is required")]
fn init_invalid_metadata_fails() {
    context("owner.near", 0);
    let mut cfg = config("owner.near");
    cfg.metadata.name = String::new();
    Contract::new(cfg, nuve_ft_id(), treasury_id());
}

#[test]
fn owner_can_repair_collection_and_token_metadata_without_burning() {
    let mut contract = contract();
    let minted = mint(&mut contract, "NuViS", "kvp_vs.tg");

    let collection_metadata = contract_metadata_with_assets();
    context("owner.near", 1);
    let updated_collection = contract.update_contract_metadata(collection_metadata.clone());
    assert_eq!(updated_collection.name, collection_metadata.name);
    assert_eq!(updated_collection.symbol, collection_metadata.symbol);
    assert_eq!(updated_collection.icon, collection_metadata.icon);
    assert_eq!(updated_collection.base_uri, collection_metadata.base_uri);
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"event\":\"contract_metadata_updated\"")));

    let stored_collection = contract.nft_metadata();
    assert_eq!(stored_collection.reference, collection_metadata.reference);
    assert_eq!(
        stored_collection.reference_hash,
        collection_metadata.reference_hash
    );

    let new_token_metadata = repaired_token_metadata();
    context("owner.near", 1);
    let updated_token =
        contract.update_token_metadata(minted.token_id.clone(), new_token_metadata.clone());
    assert_eq!(updated_token.token_id, minted.token_id);
    assert_eq!(updated_token.owner_id, account("kvp_vs.tg"));
    assert_eq!(updated_token.metadata, new_token_metadata);
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"event\":\"token_metadata_updated\"")));

    let stored_token = contract.nft_token("1".to_string()).unwrap();
    assert_eq!(
        stored_token.metadata.media,
        Some("ipfs://bafybeicz63lmkptgvkbf4oomq53qlxuyngcuelwp6rvhveuejdz6shfx24".to_string())
    );
    assert_eq!(
        stored_token.metadata.extra,
        Some("{\"type\":\"Mascot\",\"character\":\"Nuvi\",\"rarity\":\"Common\"}".to_string())
    );
}

#[test]
fn owner_can_update_and_clear_token_royalty() {
    let mut contract = contract();
    let minted = mint(&mut contract, "NuViS", "kvp_vs.tg");
    let royalty = RoyaltyConfig {
        payout: HashMap::from([(account("treasury.nuvelor.near"), 500)]),
    };

    context("owner.near", 1);
    let updated = contract.update_token_royalty(minted.token_id.clone(), Some(royalty.clone()));
    assert_eq!(updated, Some(royalty));
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"event\":\"token_royalty_updated\"")));

    let payout = contract.nft_payout(minted.token_id.clone(), U128(10_000), 10);
    assert_eq!(payout.payout[&account("treasury.nuvelor.near")], U128(500));
    assert_eq!(payout.payout[&account("kvp_vs.tg")], U128(9_500));

    context("owner.near", 1);
    let cleared = contract.update_token_royalty(minted.token_id.clone(), None);
    assert!(cleared.is_none());

    let payout = contract.nft_payout(minted.token_id, U128(10_000), 10);
    assert_eq!(payout.payout.len(), 1);
    assert_eq!(payout.payout[&account("kvp_vs.tg")], U128(10_000));
}

#[test]
fn admin_and_permissions_control_mint_and_burn() {
    let mut contract = contract();

    context("owner.near", 1);
    contract.add_admin(account("admin.near"));
    assert!(contract.is_admin(account("admin.near")));

    context("admin.near", 10u128.pow(24));
    let admin_token = contract.nft_mint(account("alice.near"), metadata("Admin"), None, None, None);

    context("owner.near", 1);
    contract.grant_permission(account("minter.near"), Permission::Mint);
    context("minter.near", 10u128.pow(24));
    let permission_token = contract.nft_mint(
        account("alice.near"),
        metadata("Permission"),
        None,
        None,
        None,
    );
    assert_eq!(admin_token.token_id, "1");
    assert_eq!(permission_token.token_id, "2");

    context("owner.near", 1);
    contract.grant_permission(account("burner.near"), Permission::Burn);
    context("burner.near", 1);
    contract.nft_burn(admin_token.token_id.clone(), None);
    assert!(contract.nft_token(admin_token.token_id).is_none());
}

#[test]
#[should_panic(expected = "Caller is not authorized to mint")]
fn unauthorized_mint_fails() {
    let mut contract = contract();
    context("alice.near", 10u128.pow(24));
    contract.nft_mint(account("alice.near"), metadata("Token"), None, None, None);
}

#[test]
#[should_panic(expected = "Caller is not owner")]
fn non_owner_cannot_update_contract_metadata() {
    let mut contract = contract();
    context("alice.near", 1);
    contract.update_contract_metadata(contract_metadata_with_assets());
}

#[test]
#[should_panic(expected = "Caller is not owner")]
fn non_owner_cannot_update_token_metadata() {
    let mut contract = contract();
    let minted = mint(&mut contract, "NuViS", "kvp_vs.tg");
    context("alice.near", 1);
    contract.update_token_metadata(minted.token_id, repaired_token_metadata());
}

#[test]
#[should_panic(expected = "Caller is not owner")]
fn non_owner_cannot_update_token_royalty() {
    let mut contract = contract();
    let minted = mint(&mut contract, "NuViS", "kvp_vs.tg");
    context("alice.near", 1);
    contract.update_token_royalty(
        minted.token_id,
        Some(RoyaltyConfig {
            payout: HashMap::from([(account("treasury.nuvelor.near"), 500)]),
        }),
    );
}

#[test]
fn mint_batch_burn_and_no_reuse_after_burn() {
    let mut contract = contract();
    context("owner.near", 10u128.pow(25));
    let minted = contract.nft_batch_mint(
        account("alice.near"),
        (0..100)
            .map(|index| MintTokenInput {
                metadata: metadata(&format!("Token {index}")),
                royalty: None,
                class: None,
            })
            .collect(),
        Some("batch".to_string()),
    );
    assert_eq!(minted.len(), 100);
    assert_eq!(minted.first().unwrap().token_id, "1");
    assert_eq!(minted.last().unwrap().token_id, "100");
    assert_eq!(contract.next_token_id, 101);
    assert_eq!(contract.nft_total_supply(), U128(100));
    assert_eq!(
        contract.nft_supply_for_owner(account("alice.near")),
        U128(100)
    );

    context("alice.near", 1);
    contract.nft_batch_burn(
        (1..=100).map(|index| index.to_string()).collect(),
        Some("burn".to_string()),
    );
    assert_eq!(contract.nft_total_supply(), U128(0));
    assert!(contract.nft_tokens(None, Some(200)).is_empty());

    context("owner.near", 10u128.pow(24));
    let next = contract.nft_mint(account("alice.near"), metadata("Next"), None, None, None);
    assert_eq!(next.token_id, "101");
}

#[test]
#[should_panic(expected = "Duplicate token IDs in batch")]
fn batch_burn_duplicate_ids_fails_before_mutation() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Token", "alice.near");
    context("alice.near", 1);
    contract.nft_batch_burn(
        vec![minted.token_id.clone(), minted.token_id],
        Some("duplicate".to_string()),
    );
}

#[test]
#[should_panic(expected = "Batch size exceeds max_batch")]
fn batch_mint_over_max_batch_fails() {
    let mut contract = contract();
    context("owner.near", 10u128.pow(24));
    contract.nft_batch_mint(
        account("alice.near"),
        (0..101)
            .map(|index| MintTokenInput {
                metadata: metadata(&format!("Token {index}")),
                royalty: None,
                class: None,
            })
            .collect(),
        None,
    );
}

#[test]
fn transfer_approval_enumeration_and_events_work() {
    let mut contract = contract();
    let minted = mint(&mut contract, "nft-1", "alice.near");

    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("market.near"), None);
    assert!(contract.nft_is_approved(minted.token_id.clone(), account("market.near"), Some(1)));
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"standard\":\"nuvelor-nft\"")
            && log.contains("\"event\":\"nft_approved\"")));

    context("market.near", 1);
    contract.nft_transfer(
        account("bob.near"),
        minted.token_id.clone(),
        Some(1),
        Some("sale".to_string()),
    );
    let token = contract.nft_token(minted.token_id).unwrap();
    assert_eq!(token.owner_id, account("bob.near"));
    assert!(token.approved_account_ids.is_empty());
    assert_eq!(
        contract.nft_supply_for_owner(account("alice.near")),
        U128(0)
    );
    assert_eq!(contract.nft_supply_for_owner(account("bob.near")), U128(1));
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"event\":\"nft_transfer\"")));
}

#[test]
fn approval_revoke_custom_events_work() {
    let mut contract = contract();
    let minted = mint(&mut contract, "nft-1", "alice.near");

    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("market.near"), None);

    context("alice.near", 1);
    contract.nft_revoke(minted.token_id.clone(), account("market.near"));
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"standard\":\"nuvelor-nft\"")
            && log.contains("\"event\":\"nft_approval_revoked\"")));

    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("market.near"), None);
    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("auction.near"), None);

    context("alice.near", 1);
    contract.nft_revoke_all(minted.token_id);
    assert!(get_logs()
        .iter()
        .any(|log| log.contains("\"standard\":\"nuvelor-nft\"")
            && log.contains("\"event\":\"nft_approvals_revoked_all\"")
            && log.contains("\"revoked_count\":2")));
}

#[test]
#[should_panic(expected = "Self-transfer is not allowed")]
fn transfer_self_fails() {
    let mut contract = contract();
    let minted = mint(&mut contract, "nft-1", "alice.near");
    context("alice.near", 1);
    contract.nft_transfer(account("alice.near"), minted.token_id, None, None);
}

#[test]
fn per_token_royalty_payout_works() {
    let mut contract = contract();
    let royalty = RoyaltyConfig {
        payout: HashMap::from([
            (account("artist.near"), 1_000),
            (account("treasury.near"), 500),
        ]),
    };
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Royalty"),
        Some(royalty),
        None,
        None,
    );

    let payout = contract.nft_payout(minted.token_id, U128(10_000), 10);
    assert_eq!(payout.payout[&account("artist.near")], U128(1_000));
    assert_eq!(payout.payout[&account("treasury.near")], U128(500));
    assert_eq!(payout.payout[&account("alice.near")], U128(8_500));
}

#[test]
fn seller_royalty_recipient_payout_is_summed() {
    let mut contract = contract();
    let royalty = RoyaltyConfig {
        payout: HashMap::from([(account("alice.near"), 1_000)]),
    };
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Royalty"),
        Some(royalty),
        None,
        None,
    );

    let payout = contract.nft_payout(minted.token_id, U128(10_000), 10);
    assert_eq!(payout.payout.len(), 1);
    assert_eq!(payout.payout[&account("alice.near")], U128(10_000));
}

#[test]
fn payout_uses_mint_time_royalty_after_config_decrease() {
    let mut contract = contract();
    let royalty = RoyaltyConfig {
        payout: HashMap::from([(account("artist.near"), 1_000)]),
    };
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Royalty"),
        Some(royalty),
        None,
        None,
    );

    context("owner.near", 1);
    contract.update_config(ContractConfigPatch {
        max_royalty_bps: Some(500),
        ..Default::default()
    });

    let payout = contract.nft_payout(minted.token_id, U128(10_000), 10);
    assert_eq!(payout.payout[&account("artist.near")], U128(1_000));
    assert_eq!(payout.payout[&account("alice.near")], U128(9_000));
}

#[test]
#[should_panic(expected = "Invalid royalty configuration")]
fn invalid_royalty_fails() {
    let mut contract = contract();
    let royalty = RoyaltyConfig {
        payout: HashMap::from([(account("artist.near"), 2_001)]),
    };
    context("owner.near", 10u128.pow(24));
    contract.nft_mint(
        account("alice.near"),
        metadata("Royalty"),
        Some(royalty),
        None,
        None,
    );
}

#[test]
#[should_panic(expected = "Invalid royalty configuration")]
fn royalty_recipients_must_leave_room_for_seller() {
    let mut contract = contract();
    let royalty = RoyaltyConfig {
        payout: (0..10)
            .map(|index| (account(&format!("r{index}.near")), 1))
            .collect(),
    };
    context("owner.near", 10u128.pow(24));
    contract.nft_mint(
        account("alice.near"),
        metadata("Royalty"),
        Some(royalty),
        None,
        None,
    );
}

#[test]
#[should_panic(expected = "Metadata extra is too large")]
fn metadata_extra_limit_enforced() {
    let mut contract = contract();
    let mut meta = metadata("Large");
    meta.extra = Some("x".repeat(1_025));
    context("owner.near", 10u128.pow(24));
    contract.nft_mint(account("alice.near"), meta, None, None, None);
}

#[test]
fn pause_flags_block_methods() {
    let mut contract = contract();
    context("owner.near", 1);
    contract.set_pause_flags(PauseFlags {
        mint_paused: true,
        ..Default::default()
    });
    context("owner.near", 10u128.pow(24));
    let mint_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_mint(account("alice.near"), metadata("Paused"), None, None, None);
    }));
    assert!(mint_result.is_err());
}

#[test]
fn storage_deposit_withdraw_unregister_rules() {
    let mut contract = contract();
    let min = contract.storage_balance_bounds().min.as_yoctonear();
    context("alice.near", min + 1_000);
    let balance = contract.storage_deposit(None, Some(false));
    assert_eq!(balance.total.as_yoctonear(), min + 1_000);

    context("alice.near", 1);
    let balance = contract.storage_withdraw(Some(U128(1_000)));
    assert_eq!(balance.total.as_yoctonear(), min);

    context("alice.near", 1);
    assert!(contract.storage_unregister(Some(false)));
    assert!(contract.storage_balance_of(account("alice.near")).is_none());
}

#[test]
#[should_panic(expected = "force=true is not supported")]
fn storage_unregister_force_true_is_not_supported() {
    let mut contract = contract();
    context("alice.near", 1);
    contract.storage_unregister(Some(true));
}

#[test]
fn migrate_empty_state_succeeds_when_old_supply_zero() {
    context("owner.near", 0);
    let old = crate::OldContractPrefix {
        owner_id: account("owner.near"),
        metadata: LazyOption::new(StorageKey::Metadata, Some(&config("owner.near").metadata)),
        tokens_by_id: LookupMap::new(StorageKey::TokensById),
        token_ids: UnorderedSet::new(StorageKey::TokenIds),
    };
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_empty_state(config("owner.near"), nuve_ft_id(), treasury_id());
    assert_eq!(migrated.state_version, STATE_VERSION_V7);
    assert_eq!(migrated.nft_total_supply(), U128(0));
}

#[test]
#[should_panic(expected = "Caller is not authorized to migrate")]
fn migrate_empty_state_rejects_current_account_predecessor() {
    context("owner.near", 0);
    let old = crate::OldContractPrefix {
        owner_id: account("owner.near"),
        metadata: LazyOption::new(StorageKey::Metadata, Some(&config("owner.near").metadata)),
        tokens_by_id: LookupMap::new(StorageKey::TokensById),
        token_ids: UnorderedSet::new(StorageKey::TokenIds),
    };
    env::state_write(&old);

    context("nft.nuvelor.near", 0);
    Contract::migrate_empty_state(config("owner.near"), nuve_ft_id(), treasury_id());
}

#[test]
#[should_panic(expected = "Old total supply must be zero")]
fn migrate_empty_state_fails_when_old_supply_nonzero() {
    context("owner.near", 0);
    let mut token_ids = UnorderedSet::new(StorageKey::TokenIds);
    token_ids.insert(&"existing".to_string());
    let old = crate::OldContractPrefix {
        owner_id: account("owner.near"),
        metadata: LazyOption::new(StorageKey::Metadata, Some(&config("owner.near").metadata)),
        tokens_by_id: LookupMap::new(StorageKey::TokensById),
        token_ids,
    };
    env::state_write(&old);

    context("owner.near", 0);
    Contract::migrate_empty_state(config("owner.near"), nuve_ft_id(), treasury_id());
}

fn token_class(class_id: &str, serial: u32) -> TokenClass {
    TokenClass {
        class_id: class_id.to_string(),
        serial,
    }
}

fn create_class(contract: &mut Contract, class_id: &str, max_supply: u32) {
    context("owner.near", 10u128.pow(24));
    contract.create_token_class(class_id.to_string(), max_supply);
}

fn clear_living_state(contract: &mut Contract) {
    let token_ids = contract.token_ids.iter().collect::<Vec<_>>();
    for token_id in token_ids {
        contract.living_state_by_id.remove(&token_id);
    }
}

fn contract_v2_from_v4(mut contract: Contract, state_version: u32) -> crate::ContractV2 {
    clear_living_state(&mut contract);

    crate::ContractV2 {
        owner_id: contract.owner_id,
        proposed_owner_id: contract.proposed_owner_id,
        metadata: contract.metadata,
        tokens_by_id: contract.tokens_by_id,
        token_ids: contract.token_ids,
        tokens_per_owner: contract.tokens_per_owner,
        token_metadata_by_id: contract.token_metadata_by_id,
        approvals_by_id: contract.approvals_by_id,
        next_approval_id_by_id: contract.next_approval_id_by_id,
        royalties_by_id: contract.royalties_by_id,
        next_token_id: contract.next_token_id,
        admins: contract.admins,
        permissions_by_account: contract.permissions_by_account,
        pause_flags: contract.pause_flags,
        config: contract.config,
        storage_deposits: contract.storage_deposits,
        state_version,
    }
}

fn contract_v3_from_v4(mut contract: Contract, state_version: u32) -> crate::ContractV3 {
    clear_living_state(&mut contract);

    crate::ContractV3 {
        owner_id: contract.owner_id,
        proposed_owner_id: contract.proposed_owner_id,
        metadata: contract.metadata,
        tokens_by_id: contract.tokens_by_id,
        token_ids: contract.token_ids,
        tokens_per_owner: contract.tokens_per_owner,
        token_metadata_by_id: contract.token_metadata_by_id,
        approvals_by_id: contract.approvals_by_id,
        next_approval_id_by_id: contract.next_approval_id_by_id,
        royalties_by_id: contract.royalties_by_id,
        next_token_id: contract.next_token_id,
        admins: contract.admins,
        permissions_by_account: contract.permissions_by_account,
        pause_flags: contract.pause_flags,
        config: contract.config,
        storage_deposits: contract.storage_deposits,
        state_version,
        class_max_supply_by_id: contract.class_max_supply_by_id,
        token_class_by_id: contract.token_class_by_id,
        class_token_by_serial: contract.class_token_by_serial,
    }
}

fn contract_v4_from_v7(contract: Contract, state_version: u32) -> crate::ContractV4 {
    crate::ContractV4 {
        owner_id: contract.owner_id,
        proposed_owner_id: contract.proposed_owner_id,
        metadata: contract.metadata,
        tokens_by_id: contract.tokens_by_id,
        token_ids: contract.token_ids,
        tokens_per_owner: contract.tokens_per_owner,
        token_metadata_by_id: contract.token_metadata_by_id,
        approvals_by_id: contract.approvals_by_id,
        next_approval_id_by_id: contract.next_approval_id_by_id,
        royalties_by_id: contract.royalties_by_id,
        next_token_id: contract.next_token_id,
        admins: contract.admins,
        permissions_by_account: contract.permissions_by_account,
        pause_flags: contract.pause_flags,
        config: contract.config,
        storage_deposits: contract.storage_deposits,
        state_version,
        class_max_supply_by_id: contract.class_max_supply_by_id,
        token_class_by_id: contract.token_class_by_id,
        class_token_by_serial: contract.class_token_by_serial,
        living_state_by_id: contract.living_state_by_id,
    }
}

fn contract_v5_from_v7(contract: Contract, state_version: u32) -> crate::ContractV5 {
    crate::ContractV5 {
        owner_id: contract.owner_id,
        proposed_owner_id: contract.proposed_owner_id,
        metadata: contract.metadata,
        tokens_by_id: contract.tokens_by_id,
        token_ids: contract.token_ids,
        tokens_per_owner: contract.tokens_per_owner,
        token_metadata_by_id: contract.token_metadata_by_id,
        approvals_by_id: contract.approvals_by_id,
        next_approval_id_by_id: contract.next_approval_id_by_id,
        royalties_by_id: contract.royalties_by_id,
        next_token_id: contract.next_token_id,
        admins: contract.admins,
        permissions_by_account: contract.permissions_by_account,
        pause_flags: contract.pause_flags,
        config: contract.config,
        storage_deposits: contract.storage_deposits,
        state_version,
        class_max_supply_by_id: contract.class_max_supply_by_id,
        token_class_by_id: contract.token_class_by_id,
        class_token_by_serial: contract.class_token_by_serial,
        living_state_by_id: contract.living_state_by_id,
        nuve_ft_id: contract.nuve_ft_id,
        treasury_id: contract.treasury_id,
        pending_living_payment_by_token: contract.pending_living_payment_by_token,
        next_living_settlement_id: contract.next_living_settlement_id,
    }
}

fn contract_v6_from_v7(contract: Contract, state_version: u32) -> crate::ContractV6 {
    crate::ContractV6 {
        owner_id: contract.owner_id,
        proposed_owner_id: contract.proposed_owner_id,
        metadata: contract.metadata,
        tokens_by_id: contract.tokens_by_id,
        token_ids: contract.token_ids,
        tokens_per_owner: contract.tokens_per_owner,
        token_metadata_by_id: contract.token_metadata_by_id,
        approvals_by_id: contract.approvals_by_id,
        next_approval_id_by_id: contract.next_approval_id_by_id,
        royalties_by_id: contract.royalties_by_id,
        next_token_id: contract.next_token_id,
        admins: contract.admins,
        permissions_by_account: contract.permissions_by_account,
        pause_flags: contract.pause_flags,
        config: contract.config,
        storage_deposits: contract.storage_deposits,
        state_version,
        class_max_supply_by_id: contract.class_max_supply_by_id,
        token_class_by_id: contract.token_class_by_id,
        class_token_by_serial: contract.class_token_by_serial,
        living_state_by_id: contract.living_state_by_id,
        nuve_ft_id: contract.nuve_ft_id,
        treasury_id: contract.treasury_id,
        pending_living_payment_by_token: contract.pending_living_payment_by_token,
        next_living_settlement_id: contract.next_living_settlement_id,
        global_treasury_total: contract.global_treasury_total,
        global_burn_total: contract.global_burn_total,
    }
}

#[test]
fn token_class_creation_owner_admin_and_views_work() {
    let mut contract = contract();

    context("owner.near", 10u128.pow(24));
    contract.create_token_class("genesis".to_string(), 555);
    assert_eq!(
        contract.get_token_class_max_supply("genesis".to_string()),
        Some(555)
    );

    context("owner.near", 1);
    contract.add_admin(account("admin.near"));
    context("admin.near", 10u128.pow(24));
    contract.create_token_class("founders.v1".to_string(), 25);
    assert_eq!(
        contract.get_token_class_max_supply("founders.v1".to_string()),
        Some(25)
    );
}

#[test]
fn mint_permission_does_not_authorize_class_creation() {
    let mut contract = contract();
    context("owner.near", 1);
    contract.grant_permission(account("minter.near"), Permission::Mint);
    context("minter.near", 10u128.pow(24));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.create_token_class("minter-only".to_string(), 10);
    }));
    assert!(result.is_err());
    assert_eq!(
        contract.get_token_class_max_supply("minter-only".to_string()),
        None
    );
}

#[test]
fn class_id_validation_rejects_invalid_identifiers() {
    for invalid in ["", "Genesis", " genesis", "genesis/1", "genesis x"] {
        let mut contract = contract();
        context("owner.near", 10u128.pow(24));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.create_token_class(invalid.to_string(), 10);
        }));
        assert!(result.is_err(), "expected invalid class_id: {invalid:?}");
    }

    let mut contract = contract();
    context("owner.near", 10u128.pow(24));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.create_token_class("a".repeat(65), 10);
    }));
    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "max_supply must be positive")]
fn class_creation_rejects_zero_supply() {
    let mut contract = contract();
    context("owner.near", 10u128.pow(24));
    contract.create_token_class("zero".to_string(), 0);
}

#[test]
#[should_panic(expected = "Token class already exists")]
fn class_creation_rejects_duplicate_id() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);
    context("owner.near", 10u128.pow(24));
    contract.create_token_class("genesis".to_string(), 20);
}

#[test]
#[should_panic(expected = "Insufficient storage balance")]
fn class_creation_requires_storage_funding() {
    let mut contract = contract();
    context("owner.near", 0);
    contract.create_token_class("genesis".to_string(), 10);
}

#[test]
fn ordinary_and_classed_mint_are_independent() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 20);

    context("owner.near", 10u128.pow(24));
    let ordinary = contract.nft_mint(
        account("alice.near"),
        metadata("Ordinary"),
        None,
        None,
        None,
    );
    assert_eq!(contract.get_token_class(ordinary.token_id.clone()), None);

    context("owner.near", 10u128.pow(24));
    let classed = contract.nft_mint(
        account("alice.near"),
        metadata("Classed"),
        None,
        None,
        Some(token_class("genesis", 10)),
    );
    assert_eq!(
        contract.get_token_class(classed.token_id.clone()),
        Some(token_class("genesis", 10))
    );
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 10),
        Some(classed.token_id)
    );
}

#[test]
fn serials_are_non_sequential_and_gaps_are_allowed() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 20);

    context("owner.near", 10u128.pow(24));
    let ten = contract.nft_mint(
        account("alice.near"),
        metadata("Ten"),
        None,
        None,
        Some(token_class("genesis", 10)),
    );
    context("owner.near", 10u128.pow(24));
    let one = contract.nft_mint(
        account("alice.near"),
        metadata("One"),
        None,
        None,
        Some(token_class("genesis", 1)),
    );

    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 10),
        Some(ten.token_id)
    );
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 1),
        Some(one.token_id)
    );
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 2),
        None
    );
}

#[test]
fn classed_mint_rejects_unknown_zero_out_of_range_and_duplicate_serials() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 2);

    for class in [
        token_class("unknown", 1),
        token_class("genesis", 0),
        token_class("genesis", 3),
    ] {
        context("owner.near", 10u128.pow(24));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.nft_mint(
                account("alice.near"),
                metadata("Invalid"),
                None,
                None,
                Some(class.clone()),
            );
        }));
        assert!(result.is_err());
    }

    context("owner.near", 10u128.pow(24));
    contract.nft_mint(
        account("alice.near"),
        metadata("Valid"),
        None,
        None,
        Some(token_class("genesis", 1)),
    );
    let next_before = contract.next_token_id;
    context("owner.near", 10u128.pow(24));
    let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_mint(
            account("alice.near"),
            metadata("Duplicate"),
            None,
            None,
            Some(token_class("genesis", 1)),
        );
    }));
    assert!(duplicate.is_err());
    assert_eq!(contract.next_token_id, next_before);
}

#[test]
fn batch_mint_supports_mixed_classes_and_preflights_duplicates() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);

    context("owner.near", 10u128.pow(24));
    let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_batch_mint(
            account("alice.near"),
            vec![
                MintTokenInput {
                    metadata: metadata("A"),
                    royalty: None,
                    class: Some(token_class("genesis", 1)),
                },
                MintTokenInput {
                    metadata: metadata("B"),
                    royalty: None,
                    class: Some(token_class("genesis", 1)),
                },
            ],
            None,
        );
    }));
    assert!(duplicate.is_err());
    assert_eq!(contract.nft_total_supply(), U128(0));
    assert_eq!(contract.next_token_id, 1);

    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_batch_mint(
        account("alice.near"),
        vec![
            MintTokenInput {
                metadata: metadata("Ordinary"),
                royalty: None,
                class: None,
            },
            MintTokenInput {
                metadata: metadata("Classed"),
                royalty: None,
                class: Some(token_class("genesis", 2)),
            },
        ],
        None,
    );
    assert_eq!(minted.len(), 2);
    assert_eq!(contract.get_token_class(minted[0].token_id.clone()), None);
    assert_eq!(
        contract.get_token_class(minted[1].token_id.clone()),
        Some(token_class("genesis", 2))
    );
}

#[test]
fn batch_preflight_rejects_invalid_metadata_before_any_mint() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);
    let mut invalid = metadata("Invalid");
    invalid.extra = Some("x".repeat(1_025));

    context("owner.near", 10u128.pow(24));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_batch_mint(
            account("alice.near"),
            vec![
                MintTokenInput {
                    metadata: metadata("Valid"),
                    royalty: None,
                    class: Some(token_class("genesis", 1)),
                },
                MintTokenInput {
                    metadata: invalid.clone(),
                    royalty: None,
                    class: None,
                },
            ],
            None,
        );
    }));
    assert!(result.is_err());
    assert_eq!(contract.nft_total_supply(), U128(0));
    assert_eq!(contract.next_token_id, 1);
}

#[test]
fn token_class_survives_transfer_approval_metadata_and_royalty_changes() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Classed"),
        None,
        None,
        Some(token_class("genesis", 4)),
    );
    let expected = token_class("genesis", 4);

    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("market.near"), None);
    assert_eq!(
        contract.get_token_class(minted.token_id.clone()),
        Some(expected.clone())
    );

    context("alice.near", 1);
    contract.nft_revoke(minted.token_id.clone(), account("market.near"));
    assert_eq!(
        contract.get_token_class(minted.token_id.clone()),
        Some(expected.clone())
    );

    context("alice.near", 10u128.pow(24));
    contract.nft_approve(minted.token_id.clone(), account("auction.near"), None);
    context("alice.near", 1);
    contract.nft_revoke_all(minted.token_id.clone());
    assert_eq!(
        contract.get_token_class(minted.token_id.clone()),
        Some(expected.clone())
    );

    context("owner.near", 1);
    contract.update_token_metadata(minted.token_id.clone(), metadata("Updated"));
    context("owner.near", 1);
    contract.update_token_royalty(
        minted.token_id.clone(),
        Some(RoyaltyConfig {
            payout: HashMap::from([(account("artist.near"), 100)]),
        }),
    );
    assert_eq!(
        contract.get_token_class(minted.token_id.clone()),
        Some(expected.clone())
    );

    context("alice.near", 1);
    contract.nft_transfer(account("bob.near"), minted.token_id.clone(), None, None);
    assert_eq!(contract.get_token_class(minted.token_id), Some(expected));
}

#[test]
fn burn_removes_live_class_but_preserves_serial_tombstone() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 20);
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Burn"),
        None,
        None,
        Some(token_class("genesis", 15)),
    );

    context("alice.near", 1);
    contract.nft_burn(minted.token_id.clone(), None);
    assert_eq!(contract.get_token_class(minted.token_id.clone()), None);
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 15),
        Some(minted.token_id.clone())
    );

    context("owner.near", 10u128.pow(24));
    let reuse = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_mint(
            account("bob.near"),
            metadata("Reuse"),
            None,
            None,
            Some(token_class("genesis", 15)),
        );
    }));
    assert!(reuse.is_err());
}

#[test]
fn batch_burn_preserves_serial_tombstones() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 20);
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_batch_mint(
        account("alice.near"),
        vec![
            MintTokenInput {
                metadata: metadata("A"),
                royalty: None,
                class: Some(token_class("genesis", 6)),
            },
            MintTokenInput {
                metadata: metadata("B"),
                royalty: None,
                class: Some(token_class("genesis", 7)),
            },
        ],
        None,
    );
    context("alice.near", 1);
    contract.nft_batch_burn(
        minted.iter().map(|token| token.token_id.clone()).collect(),
        None,
    );
    assert_eq!(contract.get_token_class(minted[0].token_id.clone()), None);
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 6),
        Some(minted[0].token_id.clone())
    );
    assert_eq!(
        contract.get_token_id_by_class_serial("genesis".to_string(), 7),
        Some(minted[1].token_id.clone())
    );
}

#[test]
fn migrate_v2_to_v7_preserves_representative_state() {
    let mut before = contract();
    let royalty = RoyaltyConfig {
        payout: HashMap::from([(account("artist.near"), 250)]),
    };

    context("owner.near", 10u128.pow(24));
    let minted = before.nft_mint(
        account("alice.near"),
        metadata("Legacy"),
        Some(royalty),
        Some("v2 fixture".to_string()),
        None,
    );
    context("alice.near", 10u128.pow(24));
    before.nft_approve(minted.token_id.clone(), account("market.near"), None);
    context("owner.near", 1);
    before.add_admin(account("admin.near"));
    context("owner.near", 1);
    before.grant_permission(account("minter.near"), Permission::Mint);
    context("owner.near", 1);
    before.propose_owner(account("future-owner.near"));
    let min = before.storage_balance_bounds().min.as_yoctonear();
    context("storer.near", min + 123);
    before.storage_deposit(None, Some(false));
    context("owner.near", 1);
    before.set_pause_flags(PauseFlags {
        transfers_paused: true,
        mint_paused: false,
        burn_paused: true,
        approvals_paused: false,
    });

    let old = contract_v2_from_v4(before, STATE_VERSION_V2);
    env::state_write(&old);
    context("owner.near", 0);
    let migrated = Contract::migrate_v2_to_v7(nuve_ft_id(), treasury_id());

    assert_eq!(migrated.state_version, STATE_VERSION_V7);
    assert_eq!(migrated.owner_id, account("owner.near"));
    assert_eq!(
        migrated.proposed_owner_id,
        Some(account("future-owner.near"))
    );
    assert_eq!(migrated.next_token_id, 2);
    assert_eq!(migrated.config.max_batch, 100);
    assert_eq!(migrated.config.max_tokens_per_view, 200);
    assert_eq!(migrated.config.max_royalty_bps, 2_000);
    assert_eq!(migrated.config.max_payout_len, 10);
    assert_eq!(migrated.config.max_metadata_extra_bytes, 1_024);
    assert!(migrated.is_admin(account("admin.near")));
    assert!(migrated.has_permission(account("minter.near"), Permission::Mint));
    assert!(migrated.pause_flags.transfers_paused);
    assert!(migrated.pause_flags.burn_paused);
    assert_eq!(
        migrated
            .storage_balance_of(account("storer.near"))
            .unwrap()
            .total
            .as_yoctonear(),
        min + 123
    );
    let token = migrated.nft_token(minted.token_id.clone()).unwrap();
    assert_eq!(token.owner_id, account("alice.near"));
    assert_eq!(token.metadata.title, Some("Legacy".to_string()));
    assert!(migrated.nft_is_approved(minted.token_id.clone(), account("market.near"), Some(1)));
    let payout = migrated.nft_payout(minted.token_id.clone(), U128(10_000), 10);
    assert_eq!(payout.payout[&account("artist.near")], U128(250));
    assert_eq!(migrated.get_token_class(minted.token_id), None);
    assert_eq!(
        migrated.get_token_class_max_supply("genesis".to_string()),
        None
    );
    assert_eq!(
        migrated.get_token_id_by_class_serial("genesis".to_string(), 1),
        None
    );
}

#[test]
#[should_panic(expected = "Caller is not authorized to migrate")]
fn migrate_v2_to_v7_rejects_wrong_caller() {
    let before = contract();
    let old = contract_v2_from_v4(before, STATE_VERSION_V2);
    env::state_write(&old);
    context("alice.near", 0);
    Contract::migrate_v2_to_v7(nuve_ft_id(), treasury_id());
}

#[test]
#[should_panic(expected = "Expected V2 state")]
fn migrate_v2_to_v7_rejects_wrong_state_version() {
    let before = contract();
    let old = contract_v2_from_v4(before, 99);
    env::state_write(&old);
    context("owner.near", 0);
    Contract::migrate_v2_to_v7(nuve_ft_id(), treasury_id());
}

#[test]
fn unauthorized_account_cannot_create_token_class() {
    let mut contract = contract();
    context("alice.near", 10u128.pow(24));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.create_token_class("unauthorized".to_string(), 10);
    }));
    assert!(result.is_err());
    assert_eq!(
        contract.get_token_class_max_supply("unauthorized".to_string()),
        None
    );
}

#[test]
fn mint_permission_can_mint_classed_token_but_not_create_class() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);
    context("owner.near", 1);
    contract.grant_permission(account("minter.near"), Permission::Mint);

    context("minter.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Permission Classed"),
        None,
        None,
        Some(token_class("genesis", 10)),
    );
    assert_eq!(
        contract.get_token_class(minted.token_id),
        Some(token_class("genesis", 10))
    );
}

#[test]
fn class_creation_can_charge_registered_storage_balance() {
    let mut contract = contract();
    let min = contract.storage_balance_bounds().min.as_yoctonear();
    context("owner.near", min);
    let before = contract
        .storage_deposit(None, Some(false))
        .total
        .as_yoctonear();

    context("owner.near", 0);
    contract.create_token_class("registered".to_string(), 10);
    let after = contract
        .storage_balance_of(account("owner.near"))
        .unwrap()
        .total
        .as_yoctonear();
    assert!(after < before);
    assert_eq!(
        contract.get_token_class_max_supply("registered".to_string()),
        Some(10)
    );
}

#[test]
fn batch_preflight_rejects_class_errors_before_any_new_mint() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 2);
    context("owner.near", 10u128.pow(24));
    contract.nft_mint(
        account("alice.near"),
        metadata("Existing"),
        None,
        None,
        Some(token_class("genesis", 1)),
    );

    for invalid_class in [
        token_class("unknown", 1),
        token_class("genesis", 3),
        token_class("genesis", 1),
    ] {
        let supply_before = contract.nft_total_supply();
        let next_before = contract.next_token_id;
        context("owner.near", 10u128.pow(24));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.nft_batch_mint(
                account("alice.near"),
                vec![
                    MintTokenInput {
                        metadata: metadata("Would mint first"),
                        royalty: None,
                        class: None,
                    },
                    MintTokenInput {
                        metadata: metadata("Invalid class"),
                        royalty: None,
                        class: Some(invalid_class.clone()),
                    },
                ],
                None,
            );
        }));
        assert!(result.is_err());
        assert_eq!(contract.nft_total_supply(), supply_before);
        assert_eq!(contract.next_token_id, next_before);
    }
}

#[test]
fn batch_preflight_rejects_invalid_royalty_before_any_mint() {
    let mut contract = contract();
    let invalid_royalty = RoyaltyConfig {
        payout: HashMap::from([(account("artist.near"), 2_001)]),
    };
    context("owner.near", 10u128.pow(24));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_batch_mint(
            account("alice.near"),
            vec![
                MintTokenInput {
                    metadata: metadata("Valid"),
                    royalty: None,
                    class: None,
                },
                MintTokenInput {
                    metadata: metadata("Invalid royalty"),
                    royalty: Some(invalid_royalty.clone()),
                    class: None,
                },
            ],
            None,
        );
    }));
    assert!(result.is_err());
    assert_eq!(contract.nft_total_supply(), U128(0));
    assert_eq!(contract.next_token_id, 1);
}

#[test]
fn token_view_shape_remains_without_class_field() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 1);
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Classed"),
        None,
        None,
        Some(token_class("genesis", 1)),
    );
    let view = contract.nft_token(minted.token_id).unwrap();
    let json = near_sdk::serde_json::to_value(view).unwrap();
    assert!(json.get("class").is_none());
    assert!(json.get("class_id").is_none());
    assert!(json.get("serial").is_none());
}

#[test]
fn transfer_call_rollback_preserves_token_class() {
    let mut contract = contract();
    create_class(&mut contract, "genesis", 10);
    context("owner.near", 10u128.pow(24));
    let minted = contract.nft_mint(
        account("alice.near"),
        metadata("Transfer call"),
        None,
        None,
        Some(token_class("genesis", 5)),
    );
    let expected = token_class("genesis", 5);

    context("alice.near", 1);
    let _promise = contract.nft_transfer_call(
        account("receiver.near"),
        minted.token_id.clone(),
        None,
        None,
        "rollback".to_string(),
    );
    assert_eq!(
        contract
            .nft_token(minted.token_id.clone())
            .unwrap()
            .owner_id,
        account("receiver.near")
    );
    assert_eq!(
        contract.get_token_class(minted.token_id.clone()),
        Some(expected.clone())
    );

    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(account("nft.nuvelor.near"))
        .predecessor_account_id(account("nft.nuvelor.near"));
    testing_env!(
        builder.build(),
        near_sdk::test_vm_config(),
        near_sdk::RuntimeFeesConfig::test(),
        HashMap::new(),
        vec![near_sdk::PromiseResult::Successful(b"false".to_vec())],
    );
    assert!(contract.nft_resolve_transfer(
        account("alice.near"),
        account("receiver.near"),
        minted.token_id.clone(),
        Some(HashMap::new()),
    ));
    assert_eq!(
        contract
            .nft_token(minted.token_id.clone())
            .unwrap()
            .owner_id,
        account("alice.near")
    );
    assert_eq!(contract.get_token_class(minted.token_id), Some(expected));
}

#[test]
fn migrate_v3_to_v7_rejects_already_v6_state() {
    let current = contract();
    env::state_write(&current);
    context("owner.near", 0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Contract::migrate_v3_to_v7(nuve_ft_id(), treasury_id());
    }));
    assert!(result.is_err());
}

#[test]
fn class_id_accepts_generic_sixty_four_byte_machine_key() {
    let mut contract = contract();
    let class_id = "a".repeat(64);
    context("owner.near", 10u128.pow(24));
    contract.create_token_class(class_id.clone(), 1);
    assert_eq!(contract.get_token_class_max_supply(class_id), Some(1));
}

#[test]
fn living_state_is_initialized_on_mint() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Living", "alice.near");

    let state = contract
        .get_living_state(minted.token_id)
        .expect("living state must exist");

    assert_eq!(state.form_id, DEFAULT_FORM_ID);
    assert_eq!(state.level, 1);
    assert_eq!(state.xp, 0);
    assert_eq!(state.specialization, None);
    assert_eq!(state.mutation_count, 0);
    assert_eq!(state.work_a_xp, 0);
    assert_eq!(state.work_b_xp, 0);
    assert_eq!(state.sponsor_spent, U128(0));
    assert_eq!(state.burner_spent, U128(0));
    assert_eq!(state.active_action, None);
    assert_eq!(state.ready_at_ns, 0);
}

#[test]
fn living_state_survives_transfer_and_is_removed_on_burn() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Living", "alice.near");
    let token_id = minted.token_id;

    let before = contract
        .get_living_state(token_id.clone())
        .expect("living state must exist");

    context("alice.near", 1);
    contract.nft_transfer(account("bob.near"), token_id.clone(), None, None);

    assert_eq!(contract.get_living_state(token_id.clone()), Some(before));

    context("bob.near", 1);
    contract.nft_burn(token_id.clone(), None);

    assert_eq!(contract.get_living_state(token_id), None);
}

#[test]
fn migrate_v3_to_v7_initializes_default_living_state() {
    let mut before = contract();
    let minted = mint(&mut before, "Legacy V3", "alice.near");
    let token_id = minted.token_id;

    let old = contract_v3_from_v4(before, STATE_VERSION_V3);
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_v3_to_v7(nuve_ft_id(), treasury_id());

    assert_eq!(migrated.state_version, STATE_VERSION_V7);

    let state = migrated
        .get_living_state(token_id)
        .expect("migrated token must receive living state");

    assert_eq!(state.form_id, DEFAULT_FORM_ID);
    assert_eq!(state.level, 1);
    assert_eq!(state.xp, 0);
    assert_eq!(state.active_action, None);
    assert_eq!(state.ready_at_ns, 0);
}

#[test]
fn living_game_config_exposes_demo_economy() {
    let contract = contract();
    let config = contract.get_living_game_config();

    assert_eq!(config.nuve_ft_id, nuve_ft_id());
    assert_eq!(config.treasury_id, treasury_id());
    assert_eq!(config.nuve_decimals, 4);
    assert_eq!(config.feed_cost, U128(5 * NUVE_UNIT));
    assert_eq!(config.work_cost, U128(10 * NUVE_UNIT));
    assert_eq!(config.feed_duration_ns, 30_000_000_000);
    assert_eq!(config.work_duration_ns, 10_000_000_000);
}

#[test]
fn sponsor_feed_payment_starts_timer_only_after_route_success() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Feed", "alice.near");
    let token_id = minted.token_id;

    context("nuve-demo.testnet", 0);
    let result =
        near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(FEED_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::Feed,
                EconomicRoute::Sponsor,
            ),
        );
    assert!(matches!(result, near_sdk::PromiseOrValue::Promise(_)));

    let pending = contract
        .get_pending_living_payment(token_id.clone())
        .expect("pending payment must exist");
    assert_eq!(pending.settlement_id, 1);
    assert_eq!(pending.payer_id, account("alice.near"));
    assert_eq!(pending.amount, U128(FEED_COST_ATOMIC));

    let before = contract.get_living_state(token_id.clone()).unwrap();
    assert_eq!(before.active_action, None);
    assert_eq!(before.sponsor_spent, U128(0));

    let callback_ts = 1_000_000_000_000;
    callback_context(callback_ts, near_sdk::PromiseResult::Successful(Vec::new()));
    let unused = contract.on_living_payment_routed(
        token_id.clone(),
        pending.settlement_id,
        U128(FEED_COST_ATOMIC),
    );

    assert_eq!(unused, U128(0));
    assert!(contract
        .get_pending_living_payment(token_id.clone())
        .is_none());

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.active_action, Some(LivingActionKind::Feed));
    assert_eq!(state.ready_at_ns, callback_ts + FEED_DURATION_NS);
    assert_eq!(state.sponsor_spent, U128(FEED_COST_ATOMIC));
    assert_eq!(state.burner_spent, U128(0));
}

#[test]
fn failed_burn_route_refunds_full_amount_and_does_not_start_work() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Work", "alice.near");
    let token_id = minted.token_id;

    context("nuve-demo.testnet", 0);
    let result =
        near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(WORK_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::WorkA,
                EconomicRoute::Burn,
            ),
        );
    assert!(matches!(result, near_sdk::PromiseOrValue::Promise(_)));

    let pending = contract
        .get_pending_living_payment(token_id.clone())
        .expect("pending payment must exist");

    callback_context(2_000_000_000_000, near_sdk::PromiseResult::Failed);
    let unused = contract.on_living_payment_routed(
        token_id.clone(),
        pending.settlement_id,
        U128(WORK_COST_ATOMIC),
    );

    assert_eq!(unused, U128(WORK_COST_ATOMIC));
    assert!(contract
        .get_pending_living_payment(token_id.clone())
        .is_none());

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.active_action, None);
    assert_eq!(state.ready_at_ns, 0);
    assert_eq!(state.sponsor_spent, U128(0));
    assert_eq!(state.burner_spent, U128(0));
}

#[test]
fn living_payment_rejects_wrong_ft_wrong_owner_and_wrong_amount() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Guarded", "alice.near");
    let token_id = minted.token_id;

    context("fake-ft.testnet", 0);
    let wrong_ft = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(FEED_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::Feed,
                EconomicRoute::Sponsor,
            ),
        );
    }));
    assert!(wrong_ft.is_err());

    context("nuve-demo.testnet", 0);
    let wrong_owner = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("bob.near"),
            U128(FEED_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::Feed,
                EconomicRoute::Sponsor,
            ),
        );
    }));
    assert!(wrong_owner.is_err());

    context("nuve-demo.testnet", 0);
    let wrong_amount = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(FEED_COST_ATOMIC - 1),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::Feed,
                EconomicRoute::Sponsor,
            ),
        );
    }));
    assert!(wrong_amount.is_err());

    assert!(contract.get_pending_living_payment(token_id).is_none());
}

#[test]
fn pending_living_payment_blocks_duplicate_payment_and_nft_burn() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Pending", "alice.near");
    let token_id = minted.token_id;

    context("nuve-demo.testnet", 0);
    let _ =
        near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(WORK_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::WorkB,
                EconomicRoute::Sponsor,
            ),
        );

    let duplicate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(WORK_COST_ATOMIC),
            living_payment_msg(
                token_id.clone(),
                LivingActionKind::WorkB,
                EconomicRoute::Sponsor,
            ),
        );
    }));
    assert!(duplicate.is_err());

    context("alice.near", 1);
    let burn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_burn(token_id.clone(), None);
    }));
    assert!(burn.is_err());
    assert!(contract.nft_token(token_id).is_some());
}

fn start_paid_living_action(
    contract: &mut Contract,
    token_id: &TokenId,
    owner_id: &str,
    action: LivingActionKind,
    route: EconomicRoute,
    callback_timestamp_ns: u64,
) {
    let amount = action_cost_atomic(action);
    context("nuve-demo.testnet", 0);
    let result =
        near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            contract,
            account(owner_id),
            U128(amount),
            living_payment_msg(token_id.clone(), action, route),
        );
    assert!(matches!(result, near_sdk::PromiseOrValue::Promise(_)));

    let pending = contract
        .get_pending_living_payment(token_id.clone())
        .expect("pending payment must exist");

    callback_context(
        callback_timestamp_ns,
        near_sdk::PromiseResult::Successful(Vec::new()),
    );
    assert_eq!(
        contract.on_living_payment_routed(token_id.clone(), pending.settlement_id, U128(amount)),
        U128(0)
    );
}

#[test]
fn ready_work_a_completion_awards_one_xp_and_clears_timer() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Worker A", "alice.near");
    let token_id = minted.token_id;

    let started_at = 5_000_000_000_000;
    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::WorkA,
        EconomicRoute::Sponsor,
        started_at,
    );

    let ready_at = contract
        .get_living_state(token_id.clone())
        .unwrap()
        .ready_at_ns;
    assert_eq!(ready_at, started_at + WORK_DURATION_NS);

    context_at("alice.near", 0, ready_at);
    let result = contract.complete_living_action(token_id.clone());
    assert!(matches!(result, near_sdk::PromiseOrValue::Value(true)));

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.xp, 1);
    assert_eq!(state.work_a_xp, 1);
    assert_eq!(state.work_b_xp, 0);
    assert_eq!(state.active_action, None);
    assert_eq!(state.ready_at_ns, 0);
}

#[test]
fn ready_work_b_completion_updates_work_b_affinity() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Worker B", "alice.near");
    let token_id = minted.token_id;

    let started_at = 6_000_000_000_000;
    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::WorkB,
        EconomicRoute::Burn,
        started_at,
    );

    let ready_at = started_at + WORK_DURATION_NS;
    context_at("alice.near", 0, ready_at);
    assert!(matches!(
        contract.complete_living_action(token_id.clone()),
        near_sdk::PromiseOrValue::Value(true)
    ));

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.xp, 1);
    assert_eq!(state.work_a_xp, 0);
    assert_eq!(state.work_b_xp, 1);
    assert_eq!(state.burner_spent, U128(WORK_COST_ATOMIC));
}

#[test]
fn feed_completion_waits_for_reward_callback_then_clears_timer() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Feeder", "alice.near");
    let token_id = minted.token_id;

    let started_at = 7_000_000_000_000;
    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::Feed,
        EconomicRoute::Sponsor,
        started_at,
    );

    let ready_at = started_at + FEED_DURATION_NS;
    context_at("alice.near", 0, ready_at);
    let result = contract.complete_living_action(token_id.clone());
    assert!(matches!(result, near_sdk::PromiseOrValue::Promise(_)));

    let before_callback = contract.get_living_state(token_id.clone()).unwrap();
    assert_eq!(before_callback.active_action, Some(LivingActionKind::Feed));
    assert_eq!(before_callback.ready_at_ns, ready_at);

    callback_context(
        ready_at,
        near_sdk::PromiseResult::Successful(b"true".to_vec()),
    );
    assert!(contract.on_feed_reward_minted(token_id.clone(), account("alice.near"), ready_at,));

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.active_action, None);
    assert_eq!(state.ready_at_ns, 0);
    assert_eq!(state.xp, 0);
}

#[test]
fn failed_feed_reward_keeps_ready_action_retryable() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Feed retry", "alice.near");
    let token_id = minted.token_id;

    let started_at = 8_000_000_000_000;
    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::Feed,
        EconomicRoute::Burn,
        started_at,
    );

    let ready_at = started_at + FEED_DURATION_NS;
    context_at("alice.near", 0, ready_at);
    assert!(matches!(
        contract.complete_living_action(token_id.clone()),
        near_sdk::PromiseOrValue::Promise(_)
    ));

    callback_context(ready_at, near_sdk::PromiseResult::Failed);
    assert!(!contract.on_feed_reward_minted(token_id.clone(), account("alice.near"), ready_at,));

    let state = contract.get_living_state(token_id.clone()).unwrap();
    assert_eq!(state.active_action, Some(LivingActionKind::Feed));
    assert_eq!(state.ready_at_ns, ready_at);

    context_at("alice.near", 0, ready_at + 1);
    assert!(matches!(
        contract.complete_living_action(token_id),
        near_sdk::PromiseOrValue::Promise(_)
    ));
}

#[test]
fn action_completion_rejects_early_or_non_owner() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Guarded completion", "alice.near");
    let token_id = minted.token_id;

    let started_at = 9_000_000_000_000;
    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::WorkA,
        EconomicRoute::Sponsor,
        started_at,
    );
    let ready_at = started_at + WORK_DURATION_NS;

    context_at("alice.near", 0, ready_at - 1);
    let early = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = contract.complete_living_action(token_id.clone());
    }));
    assert!(early.is_err());

    context_at("bob.near", 0, ready_at);
    let non_owner = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = contract.complete_living_action(token_id.clone());
    }));
    assert!(non_owner.is_err());

    let state = contract.get_living_state(token_id).unwrap();
    assert_eq!(state.xp, 0);
    assert_eq!(state.active_action, Some(LivingActionKind::WorkA));
}

#[test]
fn active_living_action_locks_transfer_and_burn() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Locked", "alice.near");
    let token_id = minted.token_id;

    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::WorkB,
        EconomicRoute::Sponsor,
        10_000_000_000_000,
    );

    context("alice.near", 1);
    let transfer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_transfer(account("bob.near"), token_id.clone(), None, None);
    }));
    assert!(transfer.is_err());

    context("alice.near", 1);
    let burn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.nft_burn(token_id.clone(), None);
    }));
    assert!(burn.is_err());

    assert_eq!(
        contract.nft_token(token_id.clone()).unwrap().owner_id,
        account("alice.near")
    );
    assert_eq!(
        contract.get_living_state(token_id).unwrap().active_action,
        Some(LivingActionKind::WorkB)
    );
}

#[test]
fn behavior_levels_match_agreed_nuve_thresholds() {
    assert_eq!(behavior_level(0), 0);
    assert_eq!(behavior_level(5 * NUVE_UNIT - 1), 0);
    assert_eq!(behavior_level(5 * NUVE_UNIT), 1);
    assert_eq!(behavior_level(20 * NUVE_UNIT - 1), 1);
    assert_eq!(behavior_level(20 * NUVE_UNIT), 2);
    assert_eq!(behavior_level(40 * NUVE_UNIT - 1), 2);
    assert_eq!(behavior_level(40 * NUVE_UNIT), 3);
    assert_eq!(behavior_level(1_000 * NUVE_UNIT), 3);
}

#[test]
fn mutation_preview_uses_work_and_behavior_weights_on_chain() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Preview", "alice.near");
    let token_id = minted.token_id;

    let mut state = contract.living_state_by_id.get(&token_id).unwrap();
    state.xp = MUTATION_THRESHOLD_XP;
    state.work_a_xp = 3;
    state.work_b_xp = 1;
    state.sponsor_spent = 20 * NUVE_UNIT;
    state.burner_spent = 40 * NUVE_UNIT;
    contract.living_state_by_id.insert(&token_id, &state);

    let preview = contract.get_mutation_preview(token_id).unwrap();
    assert!(preview.eligible);
    assert_eq!(preview.current_form_id, BASIC_FORM_ID);
    assert_eq!(preview.mutation_stage, 0);
    assert_eq!(preview.threshold_xp, 5);
    assert_eq!(preview.sponsor_level, 2);
    assert_eq!(preview.burner_level, 3);
    assert_eq!(preview.form_a_weight, 80);
    assert_eq!(preview.form_b_weight, 45);
    assert_eq!(preview.form_a_probability_bps, 6_400);
    assert_eq!(preview.form_b_probability_bps, 3_600);
}

#[test]
fn first_mutation_preserves_token_identity_and_xp_and_is_single_use() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Mutant", "alice.near");
    let token_id = minted.token_id;

    let mut state = contract.living_state_by_id.get(&token_id).unwrap();
    state.xp = MUTATION_THRESHOLD_XP;
    state.work_a_xp = 4;
    state.work_b_xp = 1;
    state.sponsor_spent = 20 * NUVE_UNIT;
    contract.living_state_by_id.insert(&token_id, &state);

    context_at("alice.near", 0, 20_000_000_000_000);
    let mutated = contract.mutate(token_id.clone());

    assert!(mutated.form_id == FORM_A_ID || mutated.form_id == FORM_B_ID);
    assert_eq!(mutated.mutation_count, 1);
    assert_eq!(mutated.xp, MUTATION_THRESHOLD_XP);
    assert_eq!(mutated.level, 1);
    assert_eq!(mutated.specialization, None);
    assert_eq!(
        contract.nft_token(token_id.clone()).unwrap().owner_id,
        account("alice.near")
    );

    context_at("alice.near", 0, 20_000_000_000_001);
    let second = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.mutate(token_id.clone());
    }));
    assert!(second.is_err());

    let form_before_transfer = contract.get_living_state(token_id.clone()).unwrap().form_id;

    context("alice.near", 1);
    contract.nft_transfer(account("bob.near"), token_id.clone(), None, None);

    assert_eq!(
        contract.nft_token(token_id.clone()).unwrap().owner_id,
        account("bob.near")
    );
    assert_eq!(
        contract.get_living_state(token_id).unwrap().form_id,
        form_before_transfer
    );
}

#[test]
fn mutation_requires_threshold_owner_and_unlocked_state() {
    let mut contract = contract();
    let minted = mint(&mut contract, "Mutation guards", "alice.near");
    let token_id = minted.token_id;

    context("alice.near", 0);
    let below_threshold = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.mutate(token_id.clone());
    }));
    assert!(below_threshold.is_err());

    let mut state = contract.living_state_by_id.get(&token_id).unwrap();
    state.xp = MUTATION_THRESHOLD_XP;
    contract.living_state_by_id.insert(&token_id, &state);

    context("bob.near", 0);
    let non_owner = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.mutate(token_id.clone());
    }));
    assert!(non_owner.is_err());

    start_paid_living_action(
        &mut contract,
        &token_id,
        "alice.near",
        LivingActionKind::WorkA,
        EconomicRoute::Sponsor,
        30_000_000_000_000,
    );

    let preview = contract.get_mutation_preview(token_id.clone()).unwrap();
    assert!(!preview.eligible);

    context_at("alice.near", 0, 30_000_000_000_001);
    let active_action = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.mutate(token_id.clone());
    }));
    assert!(active_action.is_err());
    assert_eq!(
        contract.get_living_state(token_id).unwrap().mutation_count,
        0
    );
}

#[test]
fn starter_nuvi_claim_is_free_once_per_account_with_basic_common_state() {
    let mut contract = contract();

    context_at("alice.near", 0, 50_000_000_000_000);
    let starter = contract.claim_starter_nuvi();

    assert_eq!(starter.owner_id, account("alice.near"));
    assert_eq!(starter.metadata.title.as_deref(), Some("NUVI Starter"));
    assert_eq!(starter.metadata.copies, Some(1));
    assert!(starter
        .metadata
        .description
        .as_deref()
        .unwrap()
        .contains("evolves with the player"));
    let extra = starter.metadata.extra.as_deref().unwrap();
    assert!(extra.contains("\"base_rarity\":\"Common\""));
    assert!(extra.contains("\"form_id\":\"basic\""));
    assert!(contract.starter_claim_used(account("alice.near")));

    let state = contract
        .get_living_state(starter.token_id.clone())
        .expect("starter LivingState must exist");
    assert_eq!(state.form_id, BASIC_FORM_ID);
    assert_eq!(state.level, 1);
    assert_eq!(state.xp, 0);
    assert_eq!(state.mutation_count, 0);
}

#[test]
fn starter_claim_never_resets_after_transfer_or_burn() {
    let mut contract = contract();

    context("alice.near", 0);
    let starter = contract.claim_starter_nuvi();
    let token_id = starter.token_id;

    context("alice.near", 1);
    contract.nft_transfer(account("bob.near"), token_id.clone(), None, None);

    context("bob.near", 1);
    contract.nft_burn(token_id, Some("starter burn test".to_string()));

    context("alice.near", 0);
    let second = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.claim_starter_nuvi();
    }));
    assert!(second.is_err());
    assert!(contract.starter_claim_used(account("alice.near")));
}

#[test]
fn different_accounts_can_each_claim_one_starter_nuvi() {
    let mut contract = contract();

    context("alice.near", 0);
    let alice = contract.claim_starter_nuvi();

    context("bob.near", 0);
    let bob = contract.claim_starter_nuvi();

    assert_ne!(alice.token_id, bob.token_id);
    assert!(contract.starter_claim_used(account("alice.near")));
    assert!(contract.starter_claim_used(account("bob.near")));
    assert_eq!(contract.nft_total_supply(), U128(2));
}

#[test]
fn starter_claim_respects_mint_pause() {
    let mut contract = contract();

    context("owner.near", 1);
    contract.set_pause_flags(PauseFlags {
        mint_paused: true,
        ..PauseFlags::default()
    });

    context("alice.near", 0);
    let paused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        contract.claim_starter_nuvi();
    }));
    assert!(paused.is_err());
    assert!(!contract.starter_claim_used(account("alice.near")));
}

#[test]
fn migrate_v6_to_v7_preserves_global_alignment_and_initializes_starter_claims() {
    let mut before = contract();
    before.global_treasury_total = 30 * NUVE_UNIT;
    before.global_burn_total = 10 * NUVE_UNIT;
    let minted = mint(&mut before, "Legacy V6", "alice.near");
    let token_id = minted.token_id;

    let old = contract_v6_from_v7(before, STATE_VERSION_V6);
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_v6_to_v7();

    assert_eq!(migrated.state_version, STATE_VERSION_V7);
    assert_eq!(
        migrated.get_global_alignment(),
        GlobalAlignmentView {
            treasury_total: U128(30 * NUVE_UNIT),
            burn_total: U128(10 * NUVE_UNIT),
            global_total: U128(40 * NUVE_UNIT),
            treasury_share_bps: 7_500,
            burn_share_bps: 2_500,
        }
    );
    assert!(migrated.get_living_state(token_id).is_some());
    assert!(!migrated.starter_claim_used(account("alice.near")));
}

#[test]
fn global_alignment_counts_only_successful_treasury_and_burn_routes() {
    let mut contract = contract();
    let sponsor = mint(&mut contract, "Global sponsor", "alice.near");
    let burner = mint(&mut contract, "Global burner", "alice.near");
    let failed = mint(&mut contract, "Global failed", "alice.near");

    start_paid_living_action(
        &mut contract,
        &sponsor.token_id,
        "alice.near",
        LivingActionKind::WorkA,
        EconomicRoute::Sponsor,
        40_000_000_000_000,
    );
    start_paid_living_action(
        &mut contract,
        &burner.token_id,
        "alice.near",
        LivingActionKind::Feed,
        EconomicRoute::Burn,
        40_000_000_000_001,
    );

    let alignment = contract.get_global_alignment();
    assert_eq!(alignment.treasury_total, U128(WORK_COST_ATOMIC));
    assert_eq!(alignment.burn_total, U128(FEED_COST_ATOMIC));
    assert_eq!(
        alignment.global_total,
        U128(WORK_COST_ATOMIC + FEED_COST_ATOMIC)
    );
    assert_eq!(alignment.treasury_share_bps, 6_666);
    assert_eq!(alignment.burn_share_bps, 3_334);

    context("nuve-demo.testnet", 0);
    let _ =
        near_contract_standards::fungible_token::receiver::FungibleTokenReceiver::ft_on_transfer(
            &mut contract,
            account("alice.near"),
            U128(WORK_COST_ATOMIC),
            living_payment_msg(
                failed.token_id.clone(),
                LivingActionKind::WorkB,
                EconomicRoute::Sponsor,
            ),
        );
    let pending = contract
        .get_pending_living_payment(failed.token_id.clone())
        .unwrap();

    callback_context(40_000_000_000_002, near_sdk::PromiseResult::Failed);
    assert_eq!(
        contract.on_living_payment_routed(
            failed.token_id,
            pending.settlement_id,
            U128(WORK_COST_ATOMIC)
        ),
        U128(WORK_COST_ATOMIC)
    );

    assert_eq!(contract.get_global_alignment(), alignment);
}

#[test]
fn migrate_v5_to_v7_preserves_game_state_and_initializes_global_alignment() {
    let mut before = contract();
    let minted = mint(&mut before, "Legacy V5", "alice.near");
    let token_id = minted.token_id.clone();

    start_paid_living_action(
        &mut before,
        &token_id,
        "alice.near",
        LivingActionKind::WorkA,
        EconomicRoute::Sponsor,
        41_000_000_000_000,
    );

    let old = contract_v5_from_v7(before, STATE_VERSION_V5);
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_v5_to_v7();

    assert_eq!(migrated.state_version, STATE_VERSION_V7);
    assert_eq!(migrated.nuve_ft_id, nuve_ft_id());
    assert_eq!(migrated.treasury_id, treasury_id());
    assert_eq!(
        migrated.get_living_state(token_id).unwrap().active_action,
        Some(LivingActionKind::WorkA)
    );
    assert_eq!(migrated.global_treasury_total, 0);
    assert_eq!(migrated.global_burn_total, 0);
    assert_eq!(migrated.get_global_alignment().global_total, U128(0));
}

#[test]
fn migrate_v4_to_v7_preserves_living_state_and_sets_game_accounts() {
    let mut before = contract();
    let minted = mint(&mut before, "Legacy V4", "alice.near");
    let token_id = minted.token_id.clone();

    let old = contract_v4_from_v7(before, STATE_VERSION_V4);
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_v4_to_v7(nuve_ft_id(), treasury_id());

    assert_eq!(migrated.state_version, STATE_VERSION_V7);
    assert_eq!(migrated.nuve_ft_id, nuve_ft_id());
    assert_eq!(migrated.treasury_id, treasury_id());
    assert!(migrated.get_living_state(token_id).is_some());
    assert_eq!(migrated.next_living_settlement_id, 1);
}

#[test]
fn legacy_batch_json_without_class_deserializes_as_none() {
    let value = near_sdk::serde_json::json!({
        "metadata": {
            "title": "Legacy JSON",
            "description": null,
            "media": null,
            "media_hash": null,
            "copies": 1,
            "issued_at": null,
            "expires_at": null,
            "starts_at": null,
            "updated_at": null,
            "extra": null,
            "reference": null,
            "reference_hash": null
        },
        "royalty": null
    });
    let input: MintTokenInput = near_sdk::serde_json::from_value(value).unwrap();
    assert!(input.class.is_none());
}
