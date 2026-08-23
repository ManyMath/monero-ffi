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
