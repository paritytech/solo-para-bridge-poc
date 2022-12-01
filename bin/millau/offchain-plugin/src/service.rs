use runtime_api::ConstructExtrinsicApi;
use futures::StreamExt;
use sc_client_api::HeaderBackend;
use sc_client_db::offchain::LocalStorage;
use sc_keystore::LocalKeystore;
use serde_json::{Map, Value};
use sp_api::{BlockT, ProvideRuntimeApi};
use std::{sync::Arc, time::Duration};
use tokio::{
	sync::Mutex,
	time::{interval, Instant, Interval},
};

use crate::{
	calls::submit_call,
	config::{config_provider::get_config, get_keypair},
	logic_provider,
	offchain::{store_commit_info, store_key},
};
use primitives::shared::{LogicProviderCall, MapToCall, OffchainCommitmentInfo, Pair};

// Start the module. To be initiated by the node's service.
// In here we use a runtime interface, which consists of some logic running on an interval
// as well as some business logic that retrieves the offchain data.
pub async fn start<B, C: 'static>(
	// Accept some closure that expects a `MapToCall`
	client: Arc<C>,
	offchain_storage: Arc<Mutex<LocalStorage>>,
	keystore: Arc<LocalKeystore>,
) where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B>,
	C::Api: ConstructExtrinsicApi<B>,
{
	// Indicate some arbitrary seconds interval, where for each "tick" the business logic will be
	// invoked
	// Fetch a JSON object with various values configured by the node operator. In addition, it
	// contains the first valid public key set by the node operator, set in the key
	// `config_account_id`.
	let config: Arc<Mutex<Map<String, Value>>> = get_config(&offchain_storage, &keystore).await;
	let interval = interval(Duration::from_secs(27));
	let start = Instant::now();
	let pair = get_keypair(&config, &keystore)
		.await
		.expect("Could not get pair from the keystore");
	futures::join!(
		run_service::<B, C>(client, pair, config.clone(), start, interval, &offchain_storage),
		crate::config::config_provider::schedule_config_update(&offchain_storage, config)
	);
}

/// In here, we run our initial logic related to the logic provider flow.
/// This includes generating the metadata id, creating metadata
/// hash, committing the hash and saving the data in the offchain storage
/// for further reveals.
async fn run_service<B, C: 'static>(
	client: Arc<C>,
	pair: Arc<Pair>,
	config: Arc<Mutex<Map<String, Value>>>,
	start: Instant,
	interval: Interval,
	offchain_storage: &Arc<Mutex<LocalStorage>>,
) where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B>,
	C::Api: ConstructExtrinsicApi<B>,
{
	tokio_stream::wrappers::IntervalStream::new(interval)
		.for_each(|now| {
			let client = client.clone();
			let pair = pair.clone();
			let config = config.clone();
			let offchain_storage = offchain_storage.clone();
			// Track the elapsed time for creation of unique ids for hashes sent to the pallet
			let elapsed = now.duration_since(start).as_secs_f32();
			let metadata_id = elapsed.trunc() as u64;
			async move {
				let config = config.lock().await;
				if let Some(reveal_hash) = logic_provider::get_data(config) {
					let (commit_hash, random_seed) =
						logic_provider::create_commit_hash(reveal_hash);

					let call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
						metadata_id,
						hash: commit_hash,
					});

					if submit_call(client, pair, call).is_ok() {
						let commit_info =
							OffchainCommitmentInfo { commit_hash, reveal_hash, random_seed };

						// Store the relevant metadata id. This will then be tracked
						// separately, the reveal window for it will be checked
						store_key(metadata_id, &offchain_storage).await;
						// Store the relevant commit information for the given metadata id
						store_commit_info(metadata_id, offchain_storage, commit_info)
							.await;
					}
				}
			}
		})
		.await
}
