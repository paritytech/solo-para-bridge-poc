use sc_client_db::offchain::LocalStorage;
use sc_keystore::LocalKeystore;
use sp_core::crypto::KeyTypeId;
use sp_keystore::SyncCryptoStore;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub const INVALID_KEY_ID: &[u8; 4] = b"pKe1";
pub const PUBLIC_KEY: &str = "5CtuA3PjYdsf3ouLSpxyYBxyC1ymbmbzW3JetdHwVFktmDpy";
pub const INVALID_KEYS: &[u8] = b"key";
pub const KEYS_LIST: &[u8] = b"test_key";
pub const TEST_KEY: &[u8] = b"test_key";
pub const TEST_KEY_STR: &str = "test_key";
pub const TEST_VALUE: &[u8] = b"test_value";
pub const TEST_VALUE_STR: &str = "test_value";

pub fn create_keystore(id: KeyTypeId, public: &[u8]) -> LocalKeystore {
	let mut path = PathBuf::new();
	// Some designated local filestore for keys specifically for this signing task
	path.push(r"./keystore-testing/broker");
	let keystore = LocalKeystore::open(path, None).unwrap();
	let _ = SyncCryptoStore::insert_unknown(
		&keystore,
		id,
		"bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice",
		public,
	);
	keystore
}

pub fn create_local_storage() -> Arc<Mutex<LocalStorage>> {
	let db = kvdb_memorydb::create(12);
	let db = sp_database::as_database(db);
	Arc::new(Mutex::new(LocalStorage::new(db as _)))
}
