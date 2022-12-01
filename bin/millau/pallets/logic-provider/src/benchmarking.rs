//! Benchmarking setup for pallet-logic-provider

use super::*;
use codec::Encode;

use crate::benchmarking::vec::Vec;
#[allow(unused)]
use crate::Pallet as Template;
use frame_benchmarking::{account, benchmarks, vec, whitelist_account};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::Bounded;

fn recreate_commit_hash(original_hash: H256, random_seed: u8) -> H256 {
	let mut combined = original_hash.encode();
	combined.push(random_seed);
	let hash = blake2_256(&combined);
	H256(hash)
}

fn get_pub_keys<T: Config>(len: u32) -> Vec<Public>
where
	T: frame_system::Config,
{
	let mut keys = Vec::new();
	for i in 0..len {
		let caller = account("account", i % 2, i);
		whitelist_account!(caller);
		T::LocalCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let encoded_caller = caller.encode();
		let public: Public = Public::try_from(&encoded_caller[..]).unwrap();
		keys.push(public);
	}
	keys
}

benchmarks! {
	commit_processing_result_hash  {
		let hash = H256([0; 32]);
		let public = sp_core::sr25519::Public::from_raw([0;32]);
		let acct = Pallet::<T>::to_account_id(public.into()).unwrap();
		whitelist_account!(acct);
		T::LocalCurrency::make_free_balance_be(&acct, BalanceOf::<T>::max_value());
		let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id: 0,
			hash,
		}).encode();

		let signature =  sp_core::sr25519::Signature::from_raw(
			hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
		);

	}: _(RawOrigin::None, commit_call, signature.into(), public.into())

	reveal_processing_result_hash {
		let hash = H256([0; 32]);
		let public = sp_core::sr25519::Public::from_raw([0;32]);
		let random_seed: u8 = 10;
		let metadata_id = 1;
		let acct = Pallet::<T>::to_account_id(public.into()).unwrap();
		whitelist_account!(acct);
		T::LocalCurrency::make_free_balance_be(&acct, BalanceOf::<T>::max_value());

		let committed_hash = recreate_commit_hash(hash, random_seed);
		let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id: metadata_id,
			hash: committed_hash,
		}).encode();

		let commit_signature =  sp_core::sr25519::Signature::from_raw(
			hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
		);
		Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), public.into()).unwrap();

		let reveal_window_starting_block = <frame_system::Pallet<T>>::block_number();
		pallet_commitments::RevealWindow::<T>::insert(
					metadata_id,
					reveal_window_starting_block,
				);

		let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
			metadata_id: metadata_id,
			reveal_hash: hash,
			random_seed
		}).encode();
		let reveal_signature = sp_core::sr25519::Signature::from_raw(
			hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
		);
}: _(RawOrigin::None, reveal_call, reveal_signature.into(), public.into())


	issue_rewards_to_some_participants {
		let metadata_id = 1;
		let s in 10 .. 1024; // total submissions
		let correct_hash = H256([0; 32]);
		let other_hash = H256([1; 32]);
		let keys = get_pub_keys::<T>(s);

		let public = sp_core::sr25519::Public::from_raw([0;32]);
		// let random_seed: u8 = 10;
		let acct = Pallet::<T>::to_account_id(public.into()).unwrap();
		whitelist_account!(acct);
		T::LocalCurrency::make_free_balance_be(&acct, BalanceOf::<T>::max_value());

		// Commit hash
		for (index, pub_key) in keys.iter().enumerate() {
			if index < keys.len() * 2 / 3  {
				let committed_hash = recreate_commit_hash(correct_hash, index as u8);

				let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
					metadata_id: metadata_id,
					hash: committed_hash,
				}).encode();

				let commit_signature =  sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
				);
				Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();
			} else if index < keys.len() * 3 / 4 {
				let committed_hash = recreate_commit_hash(other_hash, index as u8);
				let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
					metadata_id: metadata_id,
					hash: committed_hash,
				}).encode();

				let commit_signature =  sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
				);
				Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();
			} else {
				let committed_hash = recreate_commit_hash(correct_hash, index as u8);
				let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
					metadata_id: metadata_id,
					hash: committed_hash,
				}).encode();

				let commit_signature =  sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
				);
				Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();
			}
		}

		let reveal_window_starting_block = <frame_system::Pallet<T>>::block_number();
		pallet_commitments::RevealWindow::<T>::insert(
					metadata_id,
					reveal_window_starting_block,
				);

		// Reveal hash
		for (index, pub_key) in keys.iter().enumerate() {
			if index < keys.len() * 2 / 3{
				let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
					metadata_id: metadata_id,
					reveal_hash: correct_hash,
					random_seed: index as u8
				}).encode();
				let reveal_signature = sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
				);

				Pallet::<T>::reveal_processing_result_hash(RawOrigin::None.into(), reveal_call, reveal_signature.into(), pub_key.clone()).unwrap();
			} else if index < keys.len() * 3/ 4  {
				let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
					metadata_id: metadata_id,
					reveal_hash: other_hash,
					random_seed: index as u8
				}).encode();
				let reveal_signature = sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
				);

				Pallet::<T>::reveal_processing_result_hash(RawOrigin::None.into(), reveal_call, reveal_signature.into(), pub_key.clone()).unwrap();
			} else {}
		}
	}: issue_rewards(RawOrigin::None, metadata_id)

	issue_rewards_to_all_participants {
		let metadata_id = 1;
		let s in 10 .. 1024; // total submissions
		let correct_hash = H256([0; 32]);
		let other_hash = H256([1; 32]);
		let keys = get_pub_keys::<T>(s);

		// Commit hash
		for (index, pub_key) in keys.iter().enumerate() {
			let committed_hash = recreate_commit_hash(correct_hash, index as u8);
			let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id: metadata_id,
				hash: committed_hash,
			}).encode();

			let commit_signature =  sp_core::sr25519::Signature::from_raw(
				hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
			);
			Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();

		}

		let reveal_window_starting_block = <frame_system::Pallet<T>>::block_number();
		pallet_commitments::RevealWindow::<T>::insert(
					metadata_id,
					reveal_window_starting_block,
				);

		// Reveal hash
		for (index, pub_key) in keys.iter().enumerate() {
			let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id: metadata_id,
				reveal_hash: correct_hash,
				random_seed: index as u8
			}).encode();
			let reveal_signature = sp_core::sr25519::Signature::from_raw(
				hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
			);
			Pallet::<T>::reveal_processing_result_hash(RawOrigin::None.into(), reveal_call, reveal_signature.into(), pub_key.clone()).unwrap();

		}
	}: issue_rewards(RawOrigin::None, metadata_id)

	set_majority_type {
		let majority_type = Majority::OneHalf;
	}: _(RawOrigin::Root, majority_type)

	resolve_metadata_dispute {
		let metadata_id = 1;
		let s in 10 .. 1024; // total submissions
		let correct_hash = H256([0; 32]);
		let other_hash = H256([1; 32]);
		let keys = get_pub_keys::<T>(s);
		// commit hash
		for (index, pub_key) in keys.iter().enumerate() {
			if index < keys.len() / 2 {
				let committed_hash = recreate_commit_hash(correct_hash, index as u8);
				let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
					metadata_id: metadata_id,
					hash: committed_hash,
				}).encode();
				let commit_signature =  sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
				);
				Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();
			} else {
				let committed_hash = recreate_commit_hash(other_hash, index as u8);
				let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
					metadata_id: metadata_id,
					hash: committed_hash,
				}).encode();
				let commit_signature =  sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("761382dc2ecc84f80fe8b1c8e193f9c950d311bc59f214ed578091f0b6173e69333c11c9076e8874fde7c67b9a7abab086b5bceb9219596b53cf847d3140ce8d")
				);
				Pallet::<T>::commit_processing_result_hash(RawOrigin::None.into(), commit_call, commit_signature.into(), pub_key.clone()).unwrap();
			}
		}

		let reveal_window_starting_block = <frame_system::Pallet<T>>::block_number();
		pallet_commitments::RevealWindow::<T>::insert(
					metadata_id,
					reveal_window_starting_block,
				);

		for (index, pub_key) in keys.iter().enumerate() {
			if index < keys.len() / 2 {
				let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
					metadata_id: metadata_id,
					reveal_hash: correct_hash,
					random_seed: index as u8
				}).encode();
				let reveal_signature = sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
				);

				Pallet::<T>::reveal_processing_result_hash(RawOrigin::None.into(), reveal_call, reveal_signature.into(), pub_key.clone()).unwrap();
			} else {
				let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
					metadata_id: metadata_id,
					reveal_hash: other_hash,
					random_seed: index as u8
				}).encode();
				let reveal_signature = sp_core::sr25519::Signature::from_raw(
					hex_literal::hex!("d25f660f3da64719b4b241fc1d50cee735b17f75e87ce576bcf7343cea8ef9132e102e57475b9e6fbb10186fcd877caf254b1244cbfd86d822717297901c268c")
				);
				Pallet::<T>::reveal_processing_result_hash(RawOrigin::None.into(), reveal_call, reveal_signature.into(), pub_key.clone()).unwrap();
			}
		}

		let _  = Pallet::<T>::issue_rewards(RawOrigin::None.into(), 1); // this will return an error
	}: _(RawOrigin::Root, metadata_id, correct_hash.into()) // resolve in favor of the correct hash
}
