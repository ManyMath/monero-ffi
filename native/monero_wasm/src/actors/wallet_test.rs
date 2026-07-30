#![cfg(test)]

use super::*;
use crate::actors::wallet::{RestoreOutputs, hydration_seq};
use crate::messages::*;
use messages::prelude::*;
use zeroize::Zeroizing;

const BIP39_SEED: &str =
    "color ranch color remove subway public water embrace before begin liberty fault";

fn make_test_output(
    tx_hash: &str,
    output_index: u8,
    amount: u64,
    block_height: u64,
) -> monero_rust::WalletOutput {
    monero_rust::WalletOutput {
        tx_hash: tx_hash.to_string(),
        output_index,
        amount,
        amount_xmr: format!("{:.12}", amount as f64 / 1_000_000_000_000.0),
        key: format!("key_{}", tx_hash),
        key_offset: format!("offset_{}", tx_hash),
        commitment_mask: format!("mask_{}", tx_hash),
        subaddress_index: None,
        payment_id: None,
        received_output_bytes: String::new(),
        block_height,
        spent: false,
        spent_height: None,
        key_image: format!("ki_{}", tx_hash),
        is_coinbase: false,
        frozen: false,
    }
}

#[tokio::test]
async fn test_wallet_actor_can_be_created() {
    let ctx = Context::new();
    let addr = ctx.address();
    let _wallet = WalletActor::new(addr);
}

#[tokio::test]
async fn test_store_outputs_message() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());

    let msg = StoreOutputs {
        seed: "test seed".to_string(),
        network: "stagenet".to_string(),
        outputs: vec![make_test_output("test_hash", 0, 1_000_000_000_000, 1000)],
        daemon_height: 1100,
        block_hashes: vec![(1000, "hash_1000".to_string())],
        hydration_seq: 0,
    };

    let test_ctx = Context::new();
    wallet.notify(msg, &test_ctx).await;
}

#[tokio::test]
async fn test_update_scan_state_message() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());

    let msg = UpdateScanState {
        is_scanning: true,
        current_height: 1050,
        target_height: 1100,
        node_url: "http://localhost:38081".to_string(),
        seed: "test seed".to_string().into(),
        passphrase: String::new().into(),
        network: "stagenet".to_string(),
        account_lookahead: 50,
        subaddress_lookahead: 0,
        accounts_to_scan: None,
    };

    let test_ctx = Context::new();
    wallet.notify(msg, &test_ctx).await;
}

#[tokio::test]
async fn test_stop_scan_message() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());

    let start_msg = UpdateScanState {
        is_scanning: true,
        current_height: 1000,
        target_height: 1100,
        node_url: "http://localhost:38081".to_string(),
        seed: "test seed".to_string().into(),
        passphrase: String::new().into(),
        network: "stagenet".to_string(),
        account_lookahead: 50,
        subaddress_lookahead: 0,
        accounts_to_scan: None,
    };

    let test_ctx = Context::new();
    wallet.notify(start_msg, &test_ctx).await;
    wallet.notify(StopScan, &test_ctx).await;
}

#[tokio::test]
async fn test_multiple_output_batches() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());
    let test_ctx = Context::new();

    wallet
        .notify(
            StoreOutputs {
                seed: "test seed".to_string(),
                network: "stagenet".to_string(),
                outputs: vec![make_test_output("hash1", 0, 1_000_000_000_000, 1000)],
                daemon_height: 1100,
                block_hashes: vec![(1000, "bh_1000".to_string())],
                hydration_seq: 0,
            },
            &test_ctx,
        )
        .await;

    wallet
        .notify(
            StoreOutputs {
                seed: "test seed".to_string(),
                network: "stagenet".to_string(),
                outputs: vec![make_test_output("hash2", 1, 2_000_000_000_000, 1050)],
                daemon_height: 1100,
                block_hashes: vec![(1050, "bh_1050".to_string())],
                hydration_seq: 0,
            },
            &test_ctx,
        )
        .await;
}

#[tokio::test]
async fn test_scan_state_transitions() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());
    let test_ctx = Context::new();

    for height in [1000u64, 1050, 1100] {
        wallet
            .notify(
                UpdateScanState {
                    is_scanning: true,
                    current_height: height,
                    target_height: 1100,
                    node_url: "http://localhost:38081".to_string(),
                    seed: "test seed".to_string().into(),
                    passphrase: String::new().into(),
                    network: "stagenet".to_string(),
                    account_lookahead: 50,
                    subaddress_lookahead: 0,
                    accounts_to_scan: None,
                },
                &test_ctx,
            )
            .await;
    }

    wallet.notify(StopScan, &test_ctx).await;
}

// ---------------------------------------------------------------------------
// BIP39 actor-level tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_store_outputs_with_bip39_seed() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());
    let test_ctx = Context::new();

    wallet
        .notify(
            StoreOutputs {
                seed: BIP39_SEED.to_string(),
                network: "mainnet".to_string(),
                outputs: vec![make_test_output("bip39_tx", 0, 500_000_000_000, 3_100_000)],
                daemon_height: 3_200_000,
                block_hashes: vec![(3_100_000, "bh_3100000".to_string())],
                hydration_seq: 0,
            },
            &test_ctx,
        )
        .await;
}

#[tokio::test]
async fn test_update_scan_state_with_bip39_seed() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());
    let test_ctx = Context::new();

    wallet
        .notify(
            UpdateScanState {
                is_scanning: true,
                current_height: 3_000_000,
                target_height: 3_200_000,
                node_url: "http://localhost:18081".to_string(),
                seed: BIP39_SEED.to_string().into(),
                passphrase: String::new().into(),
                network: "mainnet".to_string(),
                account_lookahead: 50,
                subaddress_lookahead: 0,
                accounts_to_scan: None,
            },
            &test_ctx,
        )
        .await;

    wallet.notify(StopScan, &test_ctx).await;
}

#[tokio::test]
async fn test_full_scan_lifecycle_with_bip39_seed() {
    let ctx = Context::new();
    let addr = ctx.address();
    let mut wallet = WalletActor::new(addr.clone());
    let test_ctx = Context::new();

    // Start scan with BIP39 seed
    wallet
        .notify(
            UpdateScanState {
                is_scanning: true,
                current_height: 3_000_000,
                target_height: 3_200_000,
                node_url: "http://localhost:18081".to_string(),
                seed: BIP39_SEED.to_string().into(),
                passphrase: String::new().into(),
                network: "mainnet".to_string(),
                account_lookahead: 50,
                subaddress_lookahead: 0,
                accounts_to_scan: None,
            },
            &test_ctx,
        )
        .await;

    // Receive outputs
    wallet
        .notify(
            StoreOutputs {
                seed: BIP39_SEED.to_string(),
                network: "mainnet".to_string(),
                outputs: vec![
                    make_test_output("bip39_tx_1", 0, 1_000_000_000_000, 3_050_000),
                    make_test_output("bip39_tx_2", 0, 2_500_000_000_000, 3_100_000),
                ],
                daemon_height: 3_200_000,
                block_hashes: vec![
                    (3_050_000, "bh_3050000".to_string()),
                    (3_100_000, "bh_3100000".to_string()),
                ],
                hydration_seq: 0,
            },
            &test_ctx,
        )
        .await;

    // Update progress
    wallet
        .notify(
            UpdateScanState {
                is_scanning: true,
                current_height: 3_200_000,
                target_height: 3_200_000,
                node_url: "http://localhost:18081".to_string(),
                seed: BIP39_SEED.to_string().into(),
                passphrase: String::new().into(),
                network: "mainnet".to_string(),
                account_lookahead: 50,
                subaddress_lookahead: 0,
                accounts_to_scan: None,
            },
            &test_ctx,
        )
        .await;

    // Stop
    wallet.notify(StopScan, &test_ctx).await;
}

// ---------------------------------------------------------------------------
// A scan belongs to the wallet it was scanned for
// ---------------------------------------------------------------------------
//
// The actor holds one wallet at a time, and a scan runs in a task that outlives the
// request that started it. Loading or switching a wallet — which the UI allows while a
// scan is running, and which the send path does before every transaction — replaces that
// state underneath the running scan. Nothing in a finished batch said which wallet it was
// scanned for, so it was applied to whichever wallet happened to be loaded when it
// arrived.

const OTHER_SEED: &str =
    "hemlock jubilee eden hacksaw boil superior inroads epoxy exhale orders cavernous \
     second brunt saved richly lower upgrade hitched launching deepest mostly playful \
     layout lower eden";

fn restore(seed: &str, outputs: Vec<monero_rust::WalletOutput>) -> RestoreOutputs {
    RestoreOutputs {
        seed: seed.to_string(),
        network: "stagenet".to_string(),
        outputs,
        daemon_height: 2000,
        current_height: 1500,
        block_hashes_json: None,
        pending_state_json: None,
    }
}

fn store(seed: &str, outputs: Vec<monero_rust::WalletOutput>, hydration_seq: u64) -> StoreOutputs {
    StoreOutputs {
        seed: seed.to_string(),
        network: "stagenet".to_string(),
        outputs,
        daemon_height: 9_000,
        block_hashes: vec![(8_000, "bh_8000".to_string())],
        hydration_seq,
    }
}

/// The batch that finishes after another wallet was loaded must not be credited to it.
#[tokio::test]
async fn a_batch_from_the_previous_wallet_is_not_applied_to_the_one_just_loaded() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    // Wallet A is loaded and a scan starts for it.
    wallet
        .notify(
            restore(BIP39_SEED, vec![make_test_output("a_out", 0, 1_000, 100)]),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    // The user switches to wallet B before that scan's batch comes back.
    wallet
        .notify(
            restore(OTHER_SEED, vec![make_test_output("b_out", 0, 2_000, 200)]),
            &test_ctx,
        )
        .await;

    // A's batch arrives.
    wallet
        .notify(
            store(
                BIP39_SEED,
                vec![make_test_output("a_late", 0, 7_000, 900)],
                scan_seq,
            ),
            &test_ctx,
        )
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    assert_eq!(
        hashes,
        vec!["b_out"],
        "wallet B must hold only its own outputs"
    );
    assert_eq!(
        data.seed.as_deref(),
        Some(OTHER_SEED),
        "a stale batch must not put the other wallet's seed back on the actor"
    );
    assert_eq!(
        wallet.daemon_height_for_test(),
        2000,
        "wallet B must not be moved to the chain position of another wallet's scan"
    );
}

/// The negative control. A guard on this path that simply refused everything would leave
/// every wallet permanently empty, which is worse than the bug — and would still pass the
/// test above.
#[tokio::test]
async fn a_batch_for_the_wallet_still_loaded_is_applied() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    wallet
        .notify(
            restore(BIP39_SEED, vec![make_test_output("a_out", 0, 1_000, 100)]),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    wallet
        .notify(
            store(
                BIP39_SEED,
                vec![make_test_output("a_late", 0, 7_000, 900)],
                scan_seq,
            ),
            &test_ctx,
        )
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let mut hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    hashes.sort();
    assert_eq!(hashes, vec!["a_late", "a_out"]);
}

/// The send path re-hydrates the actor with the wallet it is already scanning before every
/// transaction. That must not cancel the scan's own results — it is why this guard is a
/// separate counter from the one that cancels a scan, rather than a `bump_generation()`
/// in the restore handler.
#[tokio::test]
async fn re_hydrating_the_same_wallet_mid_scan_does_not_discard_its_batches() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    wallet
        .notify(
            restore(BIP39_SEED, vec![make_test_output("a_out", 0, 1_000, 100)]),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    // A scan started, then the send path re-hydrated the *same* wallet, then the batch
    // came back. Its outputs are this wallet's and belong in it.
    wallet
        .notify(
            restore(BIP39_SEED, vec![make_test_output("a_out", 0, 1_000, 100)]),
            &test_ctx,
        )
        .await;
    wallet
        .notify(
            store(
                BIP39_SEED,
                vec![make_test_output("a_late", 0, 7_000, 900)],
                scan_seq,
            ),
            &test_ctx,
        )
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    assert!(
        hashes.contains(&"a_late"),
        "the wallet's own scan results were dropped: {hashes:?}"
    );
}

// ---------------------------------------------------------------------------
// The reorg path is the other half of the same batch
// ---------------------------------------------------------------------------
//
// When a batch comes back the scan task takes one of two exits: `StoreOutputs`, or —
// if its block hashes disagree with what the wallet has recorded — `HandleReorg`. Only
// the first was bound to the wallet the scan was started for. `HandleReorg` writes to
// `core_state` directly (a rollback, then this batch's outputs, hashes and key images),
// so the guard above never covered it. Its rollback *removes* outputs, so applying
// another wallet's reorg does not merely credit the wrong wallet: it deletes from it.

const B_CHAIN_JSON: &str = r#"{"hashes":{"8000":"old_hash_8000"},"genesis_hash":null}"#;

fn restore_at_height(
    seed: &str,
    outputs: Vec<monero_rust::WalletOutput>,
    current_height: u64,
) -> RestoreOutputs {
    RestoreOutputs {
        seed: seed.to_string(),
        network: "stagenet".to_string(),
        outputs,
        daemon_height: 8_100,
        current_height,
        block_hashes_json: Some(B_CHAIN_JSON.to_string()),
        pending_state_json: None,
    }
}

/// A batch whose block 8000 does not hash to what the wallet recorded there: a reorg.
fn reorg_batch(output_tx_hash: &str) -> Vec<monero_rust::BlockScanResult> {
    vec![monero_rust::BlockScanResult {
        block_height: 8_000,
        block_hash: "new_hash_8000".to_string(),
        block_timestamp: 1_700_000_000,
        tx_count: 1,
        outputs: vec![make_test_output(output_tx_hash, 0, 7_000, 8_000)],
        daemon_height: 8_100,
        spent_key_images: Vec::new(),
        spent_key_image_tx_hashes: Vec::new(),
    }]
}

fn handle_reorg(
    seed: &str,
    batch_results: Vec<monero_rust::BlockScanResult>,
    hydration_seq: u64,
) -> HandleReorg {
    HandleReorg {
        batch_results,
        accounts_to_scan: None,
        target_height: 8_100,
        batch_start_height: 8_000,
        node_url: "http://127.0.0.1:1".to_string(),
        seed: Zeroizing::new(seed.to_string()),
        passphrase: Zeroizing::new(String::new()),
        network: "stagenet".to_string(),
        account_lookahead: 0,
        subaddress_lookahead: 0,
        hydration_seq,
    }
}

/// A reorg found by the previous wallet's scan must not roll back the one just loaded.
#[tokio::test]
async fn a_reorg_from_the_previous_wallet_is_not_applied_to_the_one_just_loaded() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    // Wallet A is loaded and a scan starts for it.
    wallet
        .notify(
            restore_at_height(BIP39_SEED, vec![make_test_output("a_out", 0, 1_000, 100)], 8_010),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    // The user switches to wallet B before that scan's batch comes back. B holds an
    // output above the fork height, and its own record of what block 8000 hashes to.
    wallet
        .notify(
            restore_at_height(
                OTHER_SEED,
                vec![make_test_output("b_out", 0, 2_000, 8_005)],
                8_010,
            ),
            &test_ctx,
        )
        .await;

    // A's batch comes back carrying a reorg.
    wallet
        .notify(handle_reorg(BIP39_SEED, reorg_batch("a_late"), scan_seq), &test_ctx)
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    assert!(
        hashes.contains(&"b_out"),
        "wallet B's own output was deleted by another wallet's reorg: {hashes:?}"
    );
    assert!(
        !hashes.contains(&"a_late"),
        "wallet B was credited with the other wallet's outputs: {hashes:?}"
    );
    assert_eq!(
        wallet.current_height_for_test(),
        8_010,
        "wallet B was rewound to a fork point it never saw"
    );
    assert_eq!(
        wallet.block_hash_for_test(8_000).as_deref(),
        Some("old_hash_8000"),
        "wallet B's chain record was overwritten from another wallet's batch"
    );
}

/// The negative control, and it carries as much weight as the test above: a guard that
/// refused every reorg would leave a wallet permanently on an orphaned chain, crediting
/// it with outputs that no longer exist. The reorg the loaded wallet's *own* scan found
/// must still be applied in full.
#[tokio::test]
async fn a_reorg_for_the_wallet_still_loaded_is_applied() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    wallet
        .notify(
            restore_at_height(
                BIP39_SEED,
                vec![
                    make_test_output("kept_below_fork", 0, 1_000, 7_900),
                    make_test_output("orphaned", 0, 2_000, 8_005),
                ],
                8_010,
            ),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    wallet
        .notify(handle_reorg(BIP39_SEED, reorg_batch("on_new_chain"), scan_seq), &test_ctx)
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let mut hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    hashes.sort();
    assert_eq!(
        hashes,
        vec!["kept_below_fork", "on_new_chain"],
        "the loaded wallet's own reorg must drop the orphaned output and take the new chain's"
    );
    assert_eq!(
        wallet.block_hash_for_test(8_000).as_deref(),
        Some("new_hash_8000"),
        "the new chain's block hash must replace the orphaned one"
    );
}

/// The second control, for the same reason `re_hydrating_the_same_wallet_mid_scan_does_not
/// _discard_its_batches` exists: the send path re-hydrates the actor with the wallet it is
/// already scanning before every transaction, and that must not turn its own reorg stale.
#[tokio::test]
async fn re_hydrating_the_same_wallet_mid_scan_does_not_discard_its_reorg() {
    let ctx = Context::new();
    let mut wallet = WalletActor::new(ctx.address());
    let test_ctx = Context::new();

    wallet
        .notify(
            restore_at_height(
                BIP39_SEED,
                vec![make_test_output("orphaned", 0, 2_000, 8_005)],
                8_010,
            ),
            &test_ctx,
        )
        .await;
    let scan_seq = hydration_seq();

    wallet
        .notify(
            restore_at_height(
                BIP39_SEED,
                vec![make_test_output("orphaned", 0, 2_000, 8_005)],
                8_010,
            ),
            &test_ctx,
        )
        .await;
    wallet
        .notify(handle_reorg(BIP39_SEED, reorg_batch("on_new_chain"), scan_seq), &test_ctx)
        .await;

    let data = wallet.handle(GetWalletData, &test_ctx).await;
    let hashes: Vec<&str> = data.outputs.iter().map(|o| o.tx_hash.as_str()).collect();
    assert!(
        hashes.contains(&"on_new_chain"),
        "the wallet's own reorg was dropped: {hashes:?}"
    );
}
