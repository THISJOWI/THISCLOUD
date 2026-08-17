pub mod backup;
pub mod init;
pub mod image;
pub mod join;
pub mod marketplace;
pub mod network;
pub mod node;
pub mod status;
pub mod storage;
pub mod update;
pub mod vm;

use std::time::Duration;

pub use backup::run_backup_command;
pub use init::run_init;
pub use image::run_image_command;
pub use join::run_join;
pub use marketplace::run_marketplace_command;
pub use network::run_network_command;
pub use node::run_node_command;
pub use status::run_status;
pub use storage::run_storage_command;
pub use update::run_update;
pub use vm::run_vm_command;

/// Build a reqwest client with a 30-second timeout for all requests.
pub fn api_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client")
}

/// Attempt to extract a human-readable error message from a failed response.
/// Tries to parse the JSON body first; falls back to the status code.
pub async fn api_error_message(resp: reqwest::Response) -> String {
    let status = resp.status();
    match resp.json::<serde_json::Value>().await {
        Ok(val) => {
            if let Some(msg) = val.get("error").and_then(|e| e.as_str()) {
                msg.to_string()
            } else {
                format!("{}", status)
            }
        }
        Err(_) => format!("{}", status),
    }
}
