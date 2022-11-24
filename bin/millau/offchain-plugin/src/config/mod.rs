pub mod config_provider;
pub mod key_mgmt;
pub mod offchain_config;
#[cfg(test)]
mod test_utils;

use crate::{config::config_provider::CONFIG_ACCOUNT_ID, PluginError};
use primitives::shared::{Pair, Public};
use sc_keystore::LocalKeystore;
use serde_json::{Map, Value};
use sp_application_crypto::Ss58Codec;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Get keypair.
///
/// This function extracts the node's keys from `LocalKeystore` (provided that the node is a logic
/// provider).
pub async fn get_keypair(
	config: &Arc<Mutex<Map<String, Value>>>,
	keystore: &Arc<LocalKeystore>,
) -> Result<Arc<Pair>, PluginError> {
	let lock = config.lock().await;
	let public = lock
		.get(CONFIG_ACCOUNT_ID)
		.ok_or(PluginError::KeyNotFound)?
		.as_str()
		.unwrap_or("Invalid public key");
	let public = Public::from_ss58check(public).map_err(|_| PluginError::KeyNotFound)?;
	let key = Arc::new(
		keystore
			.key_pair::<Pair>(&public)
			.map_err(|_| PluginError::KeystoreError)?
			.ok_or(PluginError::KeyNotFound)?,
	);
	Ok(key)
}
