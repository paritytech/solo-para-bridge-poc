#![cfg_attr(not(feature = "std"), no_std)]

use core::time::Duration;
use sp_application_crypto::{sr25519, KeyTypeId};

// This file's contents are shared across runtime and client.
// Therefore, changes to types in this file will almost certainly necessitate
// a full node upgrade.

/// This constant defines how often we send processed
/// metadata hashes & update the offchain config.
pub const PROCESSING_INTERVAL: Duration = Duration::from_secs(13);

pub const PUBLIC_KEY_TYPE_ID: KeyTypeId = KeyTypeId(*b"pubK");
sp_application_crypto::app_crypto!(sr25519, PUBLIC_KEY_TYPE_ID);
// Types shared across runtime and client
pub mod shared {
	use codec::{Decode, Encode};
	use scale_info::TypeInfo;
	use sp_application_crypto::{sr25519, KeyTypeId};
	use sp_runtime::Perbill;

	pub type Hash = sp_core::H256;
	pub type MetadataId = u64;
	pub type BlockNumber = u64;

	pub const PUBLIC_KEY_TYPE_ID: KeyTypeId = KeyTypeId(*b"pubK");
	sp_application_crypto::app_crypto!(sr25519, PUBLIC_KEY_TYPE_ID);

	#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
	pub enum LogicProviderCall {
		CommitHash {
			metadata_id: MetadataId,
			hash: Hash,
		},
		RevealHash {
			reveal_hash: Hash,
			random_seed: u8,
			metadata_id: MetadataId,
			// public: Public,
		},
	}

	#[derive(Decode, Encode, Debug)]
	pub struct OffchainCommitmentInfo {
		pub reveal_hash: Hash,
		pub commit_hash: Hash,
		pub random_seed: u8,
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
	pub enum MapToCall {
		LogicProviderCall(LogicProviderCall),
	}

	// Constants
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
}

// Information shared with the client only
pub mod client {
	pub const TRACKED_STORAGE_KEYS: &[u8] = b"tracked_keys";
}
