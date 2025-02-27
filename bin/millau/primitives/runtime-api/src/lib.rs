// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use primitives::shared::{BlockNumber, MetadataId, Public, Signature};
use sp_core::sp_std::vec::Vec;

#[derive(Encode, Decode, PartialEq, Debug)]
pub enum Error {
	AccountConversion,
}

sp_api::decl_runtime_apis! {
	pub trait ConstructExtrinsicApi {
		fn submit_unchecked_extrinsic(
			mapped_call: Vec<u8>,
			signature: Signature,
			public: Public,
		) -> Result<(), ()>;
	}

	pub trait StorageQueryApi {
		fn get_reveal_window(
			metadata_id: MetadataId,
		) -> Option<(BlockNumber, BlockNumber)>;
	}
}
