// Expose a public function for retrieving the JSON object with all configuration and public key

use super::{
	key_mgmt::{get_public_key, KeyError},
	offchain_config::{build_offchain_config, OffchainError},
};

use core::time::Duration;
use primitives::{
	shared::{Public, PUBLIC_KEY_TYPE_ID},
	PROCESSING_INTERVAL,
};
use sc_client_db::offchain::LocalStorage;
use sc_keystore::LocalKeystore;
use serde_json::{Map, Value};

use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
pub const CONFIG_ACCOUNT_ID: &str = "config_account_id";

#[derive(Debug, PartialEq)]
pub enum ConfigError {
	/// Unable to convert public key to json string
	JSONConversionError,
	/// Error related to Keystore
	KeystoreError(KeyError),
	/// Error related to offchain
	OffchainError(OffchainError),
}

/// Extracts combined configuration(from keystore and offchain storage) wrapped under json object.
pub async fn get_config(
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	keystore: &Arc<LocalKeystore>,
) -> Arc<Mutex<Map<String, Value>>> {
	let json_config: Arc<Mutex<Map<String, Value>>> = Arc::new(Mutex::new(Map::new()));
	let public = get_key_info(keystore).await;
	set_offchain_data(offchain_storage, json_config.clone()).await;

	if let Ok(value) = serde_json::to_value(public) {
		json_config.lock().await.insert(CONFIG_ACCOUNT_ID.to_string(), value);
	}

	json_config
}

/// Returns KeyInfo once available else asynchronously waits for it.
async fn get_key_info(keystore: &Arc<LocalKeystore>) -> Public {
	loop {
		if let Ok(key_info) = get_public_key(PUBLIC_KEY_TYPE_ID, keystore).await {
			return key_info
		}
		log::info!(target: "runtime::runtime-plugin", "Asynchronously waiting for public key");
		tokio::time::sleep(Duration::from_secs(15)).await;
	}
}

/// Returns offchain configuration once available else asynchronously waits for it.
pub async fn set_offchain_data(
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	json_config: Arc<Mutex<Map<String, Value>>>,
) -> Arc<Mutex<Map<String, Value>>> {
	loop {
		if (build_offchain_config(offchain_storage, json_config.clone()).await).is_ok() {
			return json_config
		}
		log::info!(target: "runtime::runtime-plugin", "Asynchronously waiting for offchain config");
		tokio::time::sleep(Duration::from_secs(15)).await;
	}
}

/// Runs config update task.
///
/// It updates the existing config, wrapped by a `Mutex`, every [`PROCESSING_INTERVAL`] seconds.
/// Other services that use the config will also be able to use the updated config.
pub async fn schedule_config_update(
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	config: Arc<Mutex<Map<String, Value>>>,
) {
	// NOTE: set the interval to the same interval as in the client module
	// This will ensure consistent value updates
	let interval = tokio::time::interval(PROCESSING_INTERVAL);
	tokio_stream::wrappers::IntervalStream::new(interval)
		.for_each(|_| {
			let config = config.clone();
			async move {
				let _ = build_offchain_config(offchain_storage, config.clone()).await.map_err(
					|e| log::error!(target: "runtime::offchain-plugin", "Error updating offchain config: {:?}", e),
				);
			}
		})
		.await;
}

#[cfg(test)]
mod tests {
	use crate::config::{
		config_provider::{get_config, CONFIG_ACCOUNT_ID},
		offchain_config::OFFCHAIN_KEY,
		test_utils::{
			create_keystore, create_local_storage, KEYS_LIST, PUBLIC_KEY, TEST_KEY, TEST_KEY_STR,
			TEST_VALUE,
		},
	};
	use primitives::shared::PUBLIC_KEY_TYPE_ID;
	use sp_core::{
		crypto::Ss58Codec,
		offchain::{OffchainStorage, STORAGE_PREFIX},
		sr25519::Public,
	};
	use std::sync::Arc;

	#[tokio::test]
	async fn test_get_config_success() {
		let storage = create_local_storage();

		let mut lock = storage.lock().await;
		lock.set(STORAGE_PREFIX, OFFCHAIN_KEY, KEYS_LIST);
		lock.set(STORAGE_PREFIX, TEST_KEY, TEST_VALUE);
		drop(lock);

		let keystore = create_keystore(
			PUBLIC_KEY_TYPE_ID,
			Public::from_ss58check(PUBLIC_KEY).unwrap().as_ref(),
		);

		let result = get_config(&storage, &Arc::new(keystore)).await;

		assert!(result.lock().await.contains_key(TEST_KEY_STR));
		assert!(result.lock().await.contains_key(CONFIG_ACCOUNT_ID));
	}
}
