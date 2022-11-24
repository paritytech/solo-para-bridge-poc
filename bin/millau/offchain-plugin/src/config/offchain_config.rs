// Use this module to import the offchain chain database APIs
// Read the configuration from the offchain database - key, value pairs
// Generate a JSON object from the key, value pairs

use sc_client_db::offchain::LocalStorage;
use serde_json::{Map, Value};
use sp_api::offchain::{OffchainStorage, STORAGE_PREFIX};
use std::{fs::File, io::Read, ops::DerefMut, str, str::from_utf8, sync::Arc};
use tokio::sync::Mutex;

pub const OFFCHAIN_KEY: &[u8] = b"keys";

#[derive(Debug, PartialEq)]
pub enum OffchainError {
	/// No off-chain storage found
	OffchainStorageNotFound,
	/// Provided key is not available in the offchain storage
	OffchainKeyNotFound,
	/// Invalid JSON Type
	JSONTypeError,
	/// Error while attempting to interpret a sequence of u8 as a string
	UTF8Error,
	/// Configuration file not found
	ConfigFileNotFound,
	/// Invalid configuration file
	InvalidConfigFile,
}

/// Extracts configuration from offchain storage and returns a result of JSON object or error.
pub async fn build_offchain_config(
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	json_config: Arc<Mutex<Map<String, Value>>>,
) -> Result<(), OffchainError> {
	// extract account_id from local keystore
	match extract_offchain_value(offchain_storage, OFFCHAIN_KEY).await {
		Ok(keys) => {
			// all keys must be gathered in one pass.
			// if some keys are missed, they will be collected in the next config updates.
			let oc_keys: Vec<&str> = keys.split(',').collect();
			let mut lock = json_config.lock().await;
			let map = lock.deref_mut();
			extract_config_values(offchain_storage, oc_keys, map).await;
			Ok(())
		},
		Err(keys_error) => {
			log::error!("{:?}: Note: Please provide a valid value to keys", keys_error);
			Err(keys_error)
		},
	}
}

/// Updates `config` with new key-value pairs, and removes processed keys from `remaining_keys`.
async fn extract_config_values(
	oc_storage: &Arc<Mutex<LocalStorage>>,
	keys: Vec<&str>,
	config: &mut Map<String, Value>,
) {
	for key in keys.iter().rev() {
		let value = extract_offchain_value(oc_storage, key.trim().as_bytes()).await;
		if value.is_err() {
			log::error!("Note: Key not found: {}", key);
			continue
		}
		let value = serde_json::to_value(value.unwrap());
		if value.is_err() {
			log::error!(
				"{}: Note: Please provide a valid value for key: {}",
				value.unwrap_err(),
				key
			);
			continue
		}
		config.insert(key.trim().to_string(), value.unwrap());
	}
}
/// Extracts value corresponding to provided key from offchain storage.
async fn extract_offchain_value<'a>(
	oc_storage: &'a Arc<Mutex<LocalStorage>>,
	key: &'a [u8],
) -> Result<String, OffchainError> {
	oc_storage
		.lock()
		.await
		.get(STORAGE_PREFIX, key)
		.and_then(|oc_data| from_utf8(oc_data.as_slice()).map(|value| value.to_string()).ok())
		.ok_or(OffchainError::OffchainKeyNotFound)
}

/// Load the config from a json file into the offchain storage.
pub fn set_offchain_config(
	mut offchain_storage: LocalStorage,
	config_path: String,
) -> Result<(), OffchainError> {
	if let Ok(mut file) = File::open(config_path) {
		let mut data = String::new();

		file.read_to_string(&mut data).map_err(|_| OffchainError::InvalidConfigFile)?;
		serde_json::from_str(data.as_str())
			.ok()
			.and_then(|json_data: Value| {
				for (key, value) in json_data.as_object()?.iter() {
					offchain_storage.set(STORAGE_PREFIX, key.as_bytes(), value.as_str()?.as_bytes())
				}
				Some(())
			})
			.ok_or(OffchainError::JSONTypeError)
	} else {
		log::error!(target: "runtime:offchain_db", "{:?}", "Configuration file not found");
		Err(OffchainError::ConfigFileNotFound)
	}
}

#[cfg(test)]
mod tests {
	use super::{build_offchain_config, set_offchain_config, OffchainError, OFFCHAIN_KEY};
	use crate::config::test_utils::{
		create_local_storage, INVALID_KEYS, KEYS_LIST, TEST_KEY, TEST_KEY_STR, TEST_VALUE,
		TEST_VALUE_STR,
	};
	use sc_client_db::offchain::LocalStorage;
	use serde_json::Map;
	use sp_core::offchain::OffchainStorage;
	use std::sync::Arc;
	use tokio::sync::Mutex;

	use super::STORAGE_PREFIX;

	#[tokio::test]
	async fn test_extract_offchain_config_success() {
		let storage = create_local_storage();

		let mut lock = storage.lock().await;
		lock.set(STORAGE_PREFIX, OFFCHAIN_KEY, KEYS_LIST);
		lock.set(STORAGE_PREFIX, TEST_KEY, TEST_VALUE);
		drop(lock);
		let config = Arc::new(Mutex::new(Map::new()));

		let _ = build_offchain_config(&storage, config.clone()).await;

		assert_eq!(
			*config.lock().await.get(TEST_KEY_STR).unwrap(),
			serde_json::to_value(TEST_VALUE_STR).unwrap()
		);
	}

	#[tokio::test]
	async fn test_extract_offchain_config_with_invalid_key() {
		let storage = create_local_storage();
		let mut lock = storage.lock().await;
		lock.set(STORAGE_PREFIX, INVALID_KEYS, TEST_KEY);
		drop(lock);
		let config = Arc::new(Mutex::new(Map::new()));

		let result = build_offchain_config(&storage, config.clone()).await;
		assert_eq!(result.err().unwrap(), OffchainError::OffchainKeyNotFound);
	}

	#[test]
	fn test_set_offchain_config_success() {
		let db = kvdb_memorydb::create(12);
		let db = sp_database::as_database(db);
		let db = LocalStorage::new(db as _);
		assert!(set_offchain_config(db, "localConfig.json".to_string()).is_ok());
	}

	#[test]
	fn test_set_offchain_config_file_failure() {
		let db = kvdb_memorydb::create(12);
		let db = sp_database::as_database(db);
		let db = LocalStorage::new(db as _);
		assert_eq!(
			set_offchain_config(db, "notHere.json".to_string()),
			Err(OffchainError::ConfigFileNotFound)
		);
	}

	#[test]
	fn test_set_offchain_config_json_failure() {
		let db = kvdb_memorydb::create(12);
		let db = sp_database::as_database(db);
		let db = LocalStorage::new(db as _);
		assert_eq!(
			set_offchain_config(db, "README.md".to_string()),
			Err(OffchainError::JSONTypeError)
		);
	}
}
