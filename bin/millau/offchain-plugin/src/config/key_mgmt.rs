// Use this module to get the keys from local keystore (key id)
// Provide some utility fns for getting public key / account id, and for signing data

use std::sync::Arc;

use primitives::shared::Public;
use sc_keystore::LocalKeystore;
use sp_application_crypto::KeyTypeId;


#[derive(Debug, PartialEq)]
pub enum KeyError {
	/// Public key is not set for the given KeyTypeId
	PubKeyNotSet,
	/// Unable to get the local keystore
	KeyStoreNotFound,
	/// Provided KeyTypeId is not found in the Keystore
	TypeIdNotFound,
	/// Error related to Keystore
	Other(String),
}

/// Extracts public key from the keystore.
pub async fn get_public_key(
	key_type_id: KeyTypeId,
	local_keystore: &Arc<LocalKeystore>,
) -> Result<Public, KeyError> {
	// Some identifier for our cryptographic key type. Doesn't need to be sr25, but can be some
	// identifier that is specific to this project
	let local_keys = CryptoStore::sr25519_public_keys(local_keystore.as_ref(), key_type_id).await;
	// if we've inserted a key into the correct keystore, we'll get the first one(just as an
	// arbitrary selection)
	if !local_keys.is_empty() {
		Ok(local_keys[0].into())
	} else {
		log::error!("{:?}", KeyError::PubKeyNotSet);
		Err(KeyError::PubKeyNotSet)
	}
}

#[cfg(test)]
mod tests {
	use super::{get_public_key, Public};
	use crate::config::{
		key_mgmt::{KeyError::PubKeyNotSet, KeyTypeId},
		test_utils::{create_keystore, INVALID_KEY_ID, PUBLIC_KEY},
	};
	use primitives::shared::PUBLIC_KEY_TYPE_ID;
	use sp_core::crypto::Ss58Codec;
	use std::sync::Arc;

	#[tokio::test]
	async fn test_get_public_key_success() {
		let keystore = create_keystore(
			PUBLIC_KEY_TYPE_ID,
			Public::from_ss58check(PUBLIC_KEY).unwrap().as_ref(),
		);

		let public = Public::from_string(PUBLIC_KEY).unwrap();
		assert_eq!(get_public_key(PUBLIC_KEY_TYPE_ID, &Arc::new(keystore)).await.unwrap(), public);
	}

	#[tokio::test]
	async fn test_get_public_key_public_key_failure() {
		// passing Public key in an invalid format
		let keystore = create_keystore(KeyTypeId(*INVALID_KEY_ID), PUBLIC_KEY.as_bytes());

		assert_eq!(
			get_public_key(KeyTypeId(*INVALID_KEY_ID), &Arc::new(keystore))
				.await
				.err()
				.unwrap(),
			PubKeyNotSet
		);
	}
}
