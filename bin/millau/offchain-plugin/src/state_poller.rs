use codec::{Decode, Encode};
use futures::StreamExt;
use sc_client_api::HeaderBackend;
use sc_client_db::offchain::LocalStorage;
use sc_keystore::LocalKeystore;
use sp_api::{ApiError, BlockT, ProvideRuntimeApi};
use sp_runtime::{
	generic,
	offchain::{OffchainStorage, STORAGE_PREFIX},
};
use std::sync::Arc;
use tokio::{
	sync::Mutex,
	time::{interval, Duration},
};

use crate::{config::get_keypair, PluginError};
use primitives::{
	client::TRACKED_STORAGE_KEYS,
	shared::{LogicProviderCall, MapToCall, OffchainCommitmentInfo, MetadataId},
};
use runtime_api::{ConstructExtrinsicApi, StorageQueryApi};

impl From<runtime_api::Error> for PluginError {
	fn from(err: runtime_api::Error) -> Self {
		match err {
			runtime_api::Error::AccountConversion => PluginError::AccountConversionError,
		}
	}
}

impl From<ApiError> for PluginError {
	fn from(_: ApiError) -> Self {
		PluginError::RuntimeApiError
	}
}

impl From<codec::Error> for PluginError {
	fn from(err: codec::Error) -> Self {
		PluginError::CodecError(err)
	}
}

/// Start a task that polls reveal window state and reveals
/// the hash when it's time.
pub async fn poll_reveal_window_state<B, C>(
	offchain_storage: Arc<Mutex<LocalStorage>>,
	client: Arc<C>,
	keystore: Arc<LocalKeystore>,
) where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	C::Api: StorageQueryApi<B> + ConstructExtrinsicApi<B>,
{
	let repeat = interval(Duration::from_secs(12));

	let offchain_storage = &offchain_storage.clone();
	tokio_stream::wrappers::IntervalStream::new(repeat)
		.for_each(|_now| {
			let client = client.clone();
			let keystore = keystore.clone();
			async move {
				let _ = check_keys(client, offchain_storage, &keystore).await;
			}
		})
		.await
}

/// A function to get [`OffchainCommitmentInfo`] for some metadata id.
async fn get_offchain_data_for_key(
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	key: &MetadataId,
) -> Option<Result<OffchainCommitmentInfo, codec::Error>> {
	let lock = offchain_storage.lock().await;
	lock.get(STORAGE_PREFIX, &key.encode())
		.map(|retrieved| OffchainCommitmentInfo::decode(&mut &retrieved[..]))
}

/// Send reveal call to the runtime.
///
/// The reveal will be sent once we get to the reveal window.
async fn send_commitment_reveal<B, C>(
	client: Arc<C>,
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	keystore: &Arc<LocalKeystore>,
	key: MetadataId,
	commit_info: &OffchainCommitmentInfo,
) -> Result<(), PluginError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	C::Api: ConstructExtrinsicApi<B> + StorageQueryApi<B>,
{
	let OffchainCommitmentInfo { reveal_hash, random_seed, .. } = commit_info;
	let config = crate::config::config_provider::get_config(offchain_storage, keystore).await;
	let pair = get_keypair(&config, keystore).await?;
	let call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
		reveal_hash: *reveal_hash,
		random_seed: *random_seed,
		metadata_id: key,
	});
	crate::calls::submit_call(client, pair, call).map_err(|_| PluginError::RuntimeApiError)?;
	Ok(())
}

/// A function to check if we are in the reveal window for some metadata id `key`.
async fn within_reveal_window_for_key<B, C>(
	key: MetadataId,
	client: &Arc<C>,
) -> Result<bool, ApiError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	C::Api: ConstructExtrinsicApi<B> + StorageQueryApi<B>,
{
	let current_block = client.info().best_number;
	let current_hash = client.info().best_hash;
	let query = client
		.runtime_api()
		.get_reveal_window(current_hash, key)?;

	if let Some((window_start, window_end)) = query {
		Ok(current_block >= window_start.into() && current_block <= window_end.into())
	} else {
		Ok(false)
	}
}

/// Check the reveal windows for all saved metadatas, and send the reveals if needed.
async fn check_keys<B, C>(
	client: Arc<C>,
	offchain_storage: &Arc<Mutex<LocalStorage>>,
	keystore: &Arc<LocalKeystore>,
) -> Result<Option<()>, PluginError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	C::Api: ConstructExtrinsicApi<B> + StorageQueryApi<B>,
{
	let keys_lock = offchain_storage.lock().await;
	if let Some(stored_keys) = keys_lock.get(STORAGE_PREFIX, TRACKED_STORAGE_KEYS) {
		drop(keys_lock);
		let mut tracked_keys = Vec::<MetadataId>::decode(&mut &stored_keys[..])?;

		let mut to_remove = Vec::new();
		for idx_rd in 0..tracked_keys.len() {
			let key = tracked_keys[idx_rd];
			let client = client.clone();
			let keystore = keystore.clone();
			if within_reveal_window_for_key(key, &client).await? {
				if let Some(commit_data) = get_offchain_data_for_key(offchain_storage, &key).await {
					let commit_data = commit_data?;
					send_commitment_reveal(
						client.clone(),
						offchain_storage,
						&keystore,
						key,
						&commit_data,
					)
					.await?;
					to_remove.push(idx_rd);
				};
			}
		}

		to_remove.into_iter().for_each(|to_remove| {
			tracked_keys.remove(to_remove);
		});

		offchain_storage.lock().await.set(
			STORAGE_PREFIX,
			TRACKED_STORAGE_KEYS,
			&tracked_keys.encode(),
		);
		Ok(Some(()))
	} else {
		Ok(None)
	}
}
