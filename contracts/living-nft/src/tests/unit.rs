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
    let mut builder = VMContextBuilder::new();
    builder
        .current_account_id(account("nft.nuvelor.near"))
        .predecessor_account_id(account(predecessor))
        .attached_deposit(NearToken::from_yoctonear(deposit));
    testing_env!(builder.build());
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
    Contract::new(config("owner.near"))
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

#[test]
fn init_success() {
    let contract = contract();
    assert_eq!(contract.owner_id, account("owner.near"));
    assert_eq!(contract.state_version, STATE_VERSION_V4);
    assert_eq!(contract.next_token_id, 1);
    assert_eq!(contract.config.max_batch, 100);
    assert_eq!(contract.config.max_tokens_per_view, 200);
    assert_eq!(contract.config.max_approvals_per_token, 10);
}

#[test]
#[should_panic(expected = "Metadata name is required")]
fn init_invalid_metadata_fails() {
    context("owner.near", 0);
    let mut cfg = config("owner.near");
    cfg.metadata.name = String::new();
    Contract::new(cfg);
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
    let migrated = Contract::migrate_empty_state(config("owner.near"));
    assert_eq!(migrated.state_version, STATE_VERSION_V4);
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
    Contract::migrate_empty_state(config("owner.near"));
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
    Contract::migrate_empty_state(config("owner.near"));
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
fn migrate_v2_to_v4_preserves_representative_state() {
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
    let migrated = Contract::migrate_v2_to_v4();

    assert_eq!(migrated.state_version, STATE_VERSION_V4);
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
fn migrate_v2_to_v4_rejects_wrong_caller() {
    let before = contract();
    let old = contract_v2_from_v4(before, STATE_VERSION_V2);
    env::state_write(&old);
    context("alice.near", 0);
    Contract::migrate_v2_to_v4();
}

#[test]
#[should_panic(expected = "Expected V2 state")]
fn migrate_v2_to_v4_rejects_wrong_state_version() {
    let before = contract();
    let old = contract_v2_from_v4(before, 99);
    env::state_write(&old);
    context("owner.near", 0);
    Contract::migrate_v2_to_v4();
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
fn migrate_v3_to_v4_rejects_already_v4_state() {
    let current = contract();
    env::state_write(&current);
    context("owner.near", 0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Contract::migrate_v3_to_v4();
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
fn migrate_v3_to_v4_initializes_default_living_state() {
    let mut before = contract();
    let minted = mint(&mut before, "Legacy V3", "alice.near");
    let token_id = minted.token_id;

    let old = contract_v3_from_v4(before, STATE_VERSION_V3);
    env::state_write(&old);

    context("owner.near", 0);
    let migrated = Contract::migrate_v3_to_v4();

    assert_eq!(migrated.state_version, STATE_VERSION_V4);

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
