// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Types used to connect to the Westend chain.

use relay_substrate_client::{Chain, ChainWithBalances, RelayChain, UnderlyingChainProvider};
use sp_core::storage::StorageKey;
use std::time::Duration;

/// Westend header id.
pub type HeaderId = relay_utils::HeaderId<bp_westend::Hash, bp_westend::BlockNumber>;

/// Westend header type used in headers sync.
pub type SyncHeader = relay_substrate_client::SyncHeader<bp_westend::Header>;

/// Westend chain definition
#[derive(Debug, Clone, Copy)]
pub struct Westend;

impl UnderlyingChainProvider for Westend {
	type Chain = bp_westend::Westend;
}

impl Chain for Westend {
	const NAME: &'static str = "Westend";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		bp_westend::BEST_FINALIZED_WESTEND_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(6);

	type SignedBlock = bp_westend::SignedBlock;
	type Call = ();
}

impl RelayChain for Westend {
	const PARAS_PALLET_NAME: &'static str = bp_westend::PARAS_PALLET_NAME;
	const PARACHAINS_FINALITY_PALLET_NAME: &'static str =
		bp_westend::WITH_WESTEND_BRIDGE_PARAS_PALLET_NAME;
}

impl ChainWithBalances for Westend {
	fn account_info_storage_key(account_id: &Self::AccountId) -> StorageKey {
		bp_westend::AccountInfoStorageMapKeyProvider::final_key(account_id)
	}
}

/// `AssetHubWestend` parachain definition
#[derive(Debug, Clone, Copy)]
pub struct AssetHubWestend;

impl UnderlyingChainProvider for AssetHubWestend {
	type Chain = bp_westend::AssetHubWestend;
}

// AssetHubWestend seems to use the same configuration as all Polkadot-like chains, so we'll use
// Westend primitives here.
impl Chain for AssetHubWestend {
	const NAME: &'static str = "AssetHubWestend";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str =
		bp_westend::BEST_FINALIZED_ASSETHUBWESTEND_HEADER_METHOD;
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_secs(6);

	type SignedBlock = bp_westend::SignedBlock;
	type Call = ();
}
