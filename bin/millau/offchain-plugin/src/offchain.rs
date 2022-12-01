use codec::{Decode, Encode};
use primitives::{client::TRACKED_STORAGE_KEYS, shared::OffchainCommitmentInfo};
use sc_client_db::offchain::LocalStorage;
use sp_core::offchain::{OffchainStorage, STORAGE_PREFIX};
use std::sync::Arc;
use tokio::sync::Mutex;

// Set a key for the given metadata that the state poller will use according to its own
// schedule
pub async fn store_key(key: u64, offchain_storage: &Arc<Mutex<LocalStorage>>) {
	let mut lock = offchain_storage.lock().await;
	// If we've already set some keys for the logic to track
	if let Some(tracked_keys) = lock.get(STORAGE_PREFIX, TRACKED_STORAGE_KEYS) {
		match Vec::<u64>::decode(&mut &tracked_keys[..]) {
			Ok(mut keys_list) =>
				if !keys_list.contains(&key) {
					keys_list.push(key);
					lock.set(STORAGE_PREFIX, TRACKED_STORAGE_KEYS, &keys_list.encode());
				},
			Err(err) => log::error!("Error when decoding storage value: {:?}", err),
		}
	} else {
		lock.set(STORAGE_PREFIX, TRACKED_STORAGE_KEYS, &vec![key].encode());
	}
}

// Store the information related to a commitment to be retrieved later at the time of the reveal
pub async fn store_commit_info(
	key: u64,
	offchain_storage: Arc<Mutex<LocalStorage>>,
	commitment_info: OffchainCommitmentInfo,
) {
	let mut lock = offchain_storage.lock().await;
	log::debug!(target: "runtime::state-poller", "Now committing data {:?} for key {}", commitment_info, key);

	if lock.get(STORAGE_PREFIX, &key.encode()).is_none() {
		log::debug!(target: "runtime::state-poller", "Now committing data {:?} for key {}", commitment_info, key);
		lock.set(STORAGE_PREFIX, &key.encode(), &commitment_info.encode())
	}
}
