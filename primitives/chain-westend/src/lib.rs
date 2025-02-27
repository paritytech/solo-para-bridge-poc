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

//! Primitives of the Westend chain.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use bp_polkadot_core::*;
use frame_support::StateVersion;

use bp_header_chain::ChainWithGrandpa;
use bp_runtime::{decl_bridge_finality_runtime_apis, Chain, ChainId, Parachain};
use frame_support::weights::Weight;

/// Westend Chain
pub struct Westend;

impl Chain for Westend {
	const ID: ChainId = *b"wend";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Nonce = Nonce;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		max_extrinsic_size()
	}

	fn max_extrinsic_weight() -> Weight {
		max_extrinsic_weight()
	}
}

impl ChainWithGrandpa for Westend {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = WITH_WESTEND_GRANDPA_PALLET_NAME;
	const MAX_AUTHORITIES_COUNT: u32 = MAX_AUTHORITIES_COUNT;
	const REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY: u32 =
		REASONABLE_HEADERS_IN_JUSTIFICATON_ANCESTRY;
	const MAX_HEADER_SIZE: u32 = MAX_HEADER_SIZE;
	const AVERAGE_HEADER_SIZE_IN_JUSTIFICATION: u32 = AVERAGE_HEADER_SIZE_IN_JUSTIFICATION;
}

/// `AssetHubWestend` parachain definition
#[derive(Debug, Clone, Copy)]
pub struct AssetHubWestend;

// AssetHubWestend seems to use the same configuration as all Polkadot-like chains, so we'll use
// Westend primitives here.
impl Chain for AssetHubWestend {
	const ID: ChainId = *b"ahwe";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Nonce = Nonce;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V0;

	fn max_extrinsic_size() -> u32 {
		Westend::max_extrinsic_size()
	}

	fn max_extrinsic_weight() -> Weight {
		Westend::max_extrinsic_weight()
	}
}

impl Parachain for AssetHubWestend {
	const PARACHAIN_ID: u32 = ASSET_HUB_WESTEND_PARACHAIN_ID;
}

/// Name of the parachains pallet at the Westend runtime.
pub const PARAS_PALLET_NAME: &str = "Paras";

/// Name of the With-Westend GRANDPA pallet instance that is deployed at bridged chains.
pub const WITH_WESTEND_GRANDPA_PALLET_NAME: &str = "BridgeWestendGrandpa";
/// Name of the With-Westend parachains bridge pallet instance that is deployed at bridged chains.
pub const WITH_WESTEND_BRIDGE_PARAS_PALLET_NAME: &str = "BridgeWestendParachains";

/// Maximal SCALE-encoded size of parachains headers that are stored at Westend `Paras` pallet.
///
/// It includes the block number and state root, so it shall be near 40 bytes, but let's have some
/// reserve.
pub const MAX_NESTED_PARACHAIN_HEAD_DATA_SIZE: u32 = 128;

/// Identifier of `AssetHubWestend` parachain at the Westend relay chain.
pub const ASSET_HUB_WESTEND_PARACHAIN_ID: u32 = 1000;

decl_bridge_finality_runtime_apis!(westend);

decl_bridge_finality_runtime_apis!(AssetHubWestend);
