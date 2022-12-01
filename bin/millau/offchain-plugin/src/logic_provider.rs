use codec::Encode;
use rand::Rng;
use serde_json::{Map, Value};
use sp_core::H256;
use sp_io::hashing::blake2_256;
use std::{fs::File, io::Read};
use tokio::sync::MutexGuard;

// JSON key of value that determines the path to retrieve the file from. We use this in this example
// to exemplify retrieval of business logic-specific values.
const JSON_PATH_KEY: &str = "local_file_path";

// Retrieve some offchain data, based on a configured value. Hash and return that value. Just for
// example's sake we will get offchain data in the form of a local file.
pub fn get_data(config: MutexGuard<Map<String, Value>>) -> Option<H256> {
	// We will not proceed if the key was not provided by the operator.
	if let Some(path_to_file) = config.get(JSON_PATH_KEY) {
		let file_data = File::open(path_to_file.as_str().expect("Invalid file path given"));
		if let Ok(mut file) = file_data {
			let mut contents = String::new();
			file.read_to_string(&mut contents).unwrap();
			let file_bytes = contents.as_bytes();
			// Return hash of the whole file. In this case we know that the `Hash` type
			// configured in the runtime is H256. This will need to be updated should the configured
			// hash type in the Runtime change.
			Some(H256::from_slice(&blake2_256(file_bytes)))
		} else {
			None
		}
	} else {
		log::info!(
			"Could not get local file path. The key {:?} should be set via RPC",
			JSON_PATH_KEY
		);
		None
	}
}

pub fn create_commit_hash(original_data: H256) -> (H256, u8) {
	let mut rng = rand::thread_rng();
	let random_seed = rng.gen::<u8>();
	let mut combined = original_data.encode();
	combined.push(random_seed);
	let hash = blake2_256(&combined);
	(H256(hash), random_seed)
}
