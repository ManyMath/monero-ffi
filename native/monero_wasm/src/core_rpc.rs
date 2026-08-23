#[cfg(target_arch = "wasm32")]
use monero_rust::monero_backend::rpc::Rpc;

pub(crate) async fn get_daemon_height(node_url: &str) -> Result<u64, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::get_daemon_height(node_url).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::daemon_height(&rpc).await
    }
}

pub(crate) async fn is_key_image_spent(
    node_url: &str,
    key_images: &[String],
) -> Result<Vec<u32>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::is_key_image_spent(node_url, key_images).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::key_image_spent_status(&rpc, key_images).await
    }
}

pub(crate) async fn fetch_blocks_batch_with_url(
    node_url: &str,
    start_height: u64,
    prune: bool,
) -> Result<monero_rust::FetchedBlocks, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::fetch_blocks_batch_with_url(node_url, start_height, prune).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::fetch_blocks_batch(&rpc, start_height, prune).await
    }
}

pub(crate) async fn fetch_blocks_batch_with_history_url(
    node_url: &str,
    start_height: u64,
    known_hashes: &[(u64, String)],
    prune: bool,
) -> Result<(monero_rust::FetchedBlocks, u64), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::fetch_blocks_batch_with_history_url(
            node_url,
            start_height,
            known_hashes,
            prune,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::fetch_blocks_batch_with_history(&rpc, start_height, known_hashes, prune).await
    }
}

pub(crate) async fn scan_block_for_outputs_with_url_and_lookahead(
    node_url: &str,
    block_height: u64,
    mnemonic: &str,
    network: &str,
    lookahead: monero_rust::Lookahead,
    passphrase: &str,
) -> Result<monero_rust::BlockScanResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::scan_block_for_outputs_with_url_and_lookahead(
            node_url,
            block_height,
            mnemonic,
            network,
            lookahead,
            passphrase,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        monero_rust::error_codes::validate_network(network).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::scanner::scan_block_for_outputs_with_lookahead(
            &rpc,
            block_height,
            mnemonic,
            network,
            lookahead,
            passphrase,
        )
        .await
    }
}

pub(crate) async fn scan_block_multi_wallet_with_url(
    node_url: &str,
    block_height: u64,
    wallets: Vec<monero_rust::WalletScanConfig>,
) -> Result<monero_rust::MultiWalletScanResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::scan_block_multi_wallet_with_url(node_url, block_height, wallets).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::scan_block_multi_wallet_wasm(&rpc, block_height, wallets).await
    }
}

pub(crate) async fn scan_mempool_for_outputs_with_account_lookahead(
    node_url: &str,
    mnemonic: &str,
    network: &str,
    account_lookahead: u32,
    subaddress_lookahead: u32,
    accounts_to_scan: Option<&[u32]>,
    passphrase: &str,
) -> Result<monero_rust::MempoolScanResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::scan_mempool_for_outputs_with_account_lookahead(
            node_url,
            mnemonic,
            network,
            account_lookahead,
            subaddress_lookahead,
            accounts_to_scan,
            passphrase,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        let lookahead = monero_rust::Lookahead {
            account: account_lookahead,
            subaddress: if subaddress_lookahead > 0 {
                subaddress_lookahead
            } else {
                monero_rust::DEFAULT_LOOKAHEAD.subaddress
            },
        };
        monero_rust::scan_mempool(
            &rpc,
            mnemonic,
            network,
            lookahead,
            accounts_to_scan,
            passphrase,
        )
        .await
    }
}

pub(crate) async fn create_transaction(
    node_url: &str,
    seed_phrase: &str,
    network: &str,
    stored_outputs: Vec<monero_rust::native::StoredOutputData>,
    recipients: &[(String, u64)],
    passphrase: &str,
) -> Result<monero_rust::native::TransactionResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::native::create_transaction(
            node_url,
            seed_phrase,
            network,
            stored_outputs,
            recipients,
            passphrase,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::native::create_transaction_with_rpc(
            &rpc,
            seed_phrase,
            network,
            stored_outputs,
            recipients,
            passphrase,
        )
        .await
    }
}

pub(crate) async fn sweep_all(
    node_url: &str,
    seed_phrase: &str,
    network: &str,
    stored_outputs: Vec<monero_rust::native::StoredOutputData>,
    destination_address: &str,
    passphrase: &str,
) -> Result<monero_rust::native::TransactionResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::native::sweep_all(
            node_url,
            seed_phrase,
            network,
            stored_outputs,
            destination_address,
            passphrase,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::native::sweep_all_with_rpc(
            &rpc,
            seed_phrase,
            network,
            stored_outputs,
            destination_address,
            passphrase,
        )
        .await
    }
}

pub(crate) async fn broadcast_transaction(
    node_url: &str,
    tx_blob_hex: &str,
    do_not_relay: bool,
) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::native::broadcast_transaction(node_url, tx_blob_hex, do_not_relay).await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::native::broadcast_transaction_with_rpc(&rpc, tx_blob_hex, do_not_relay).await
    }
}

pub(crate) async fn create_unsigned_transaction(
    node_url: &str,
    view_key_hex: &str,
    pub_spend_key_hex: &str,
    network: &str,
    stored_outputs: Vec<monero_rust::native::StoredOutputData>,
    recipients: &[(String, u64)],
    max_fee_per_weight: Option<u64>,
) -> Result<monero_rust::native::UnsignedTransactionResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        monero_rust::native::create_unsigned_transaction(
            node_url,
            view_key_hex,
            pub_spend_key_hex,
            network,
            stored_outputs,
            recipients,
            max_fee_per_weight,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    {
        monero_rust::error_codes::validate_node_url(node_url).map_err(|e| e.message)?;
        let rpc = Rpc::new_with_connection(crate::rpc_adapter::WasmRpcAdapter::new(
            node_url.to_string(),
        ));
        monero_rust::native::create_unsigned_transaction_with_rpc(
            &rpc,
            view_key_hex,
            pub_spend_key_hex,
            network,
            stored_outputs,
            recipients,
            max_fee_per_weight,
        )
        .await
    }
}
