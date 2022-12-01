use crate::{
	mock::*, Config, Error, Majority, MajorityType, MetadataId, Pallet, RoundState,
	RoundStates,
};
use frame_support::{
	assert_err, assert_ok,
	dispatch::RawOrigin,
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement, Len, WithdrawReasons},
};

use pallet_commitments::Commitment;
use primitives::shared::{LogicProviderCall, MapToCall};
use sp_core::Pair;
use sp_io::hashing::blake2_256;

fn get_hashes() -> (sp_core::H256, sp_core::H256) {
	(H256::from_low_u64_be(42_u64), H256::from_low_u64_be(43_u64))
}

pub fn create_commit_hash(original_data: H256, random_seed: u8) -> H256 {
	let mut combined = original_data.encode();
	combined.push(random_seed);
	let hash = blake2_256(&combined);
	H256(hash)
}

#[test]
fn hash_gets_added_to_storage_on_commit() {
	let (mut test_externalities, test_keys) = new_test_ext();
	test_externalities.execute_with(|| {
		// execute the call from the on-chain context
		let metadata_id: MetadataId = 0;
		let test_key = &test_keys[0];

		let (test_hash, ..) = get_hashes();
		let random_seed = 1;
		let commit_hash = create_commit_hash(test_hash, random_seed);

		let account = get_account_from_public(test_key.public());
		let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);

		let call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id,
			hash: commit_hash,
		})
		.encode();

		let signature = test_key.sign(&call);

		// checking the tokens before locking
		assert_ok!(<Test as Config>::LocalCurrency::ensure_can_withdraw(
			&account,
			FundsToLock::get(),
			WithdrawReasons::all(),
			free_balance
		));

		assert_ok!(Pallet::<Test>::commit_processing_result_hash(
			RawOrigin::None.into(),
			call,
			signature,
			test_key.public(),
		));

		let new_balance_after_lock = free_balance.checked_sub(FundsToLock::get()).unwrap();
		// checking tokens after locking
		assert!(<Test as Config>::LocalCurrency::withdraw(
			&account,
			free_balance,
			WithdrawReasons::all(),
			ExistenceRequirement::KeepAlive
		)
		.is_err());
		assert_ok!(<Test as Config>::LocalCurrency::withdraw(
			&account,
			new_balance_after_lock,
			WithdrawReasons::all(),
			ExistenceRequirement::KeepAlive
		));
		let participant_submissions =
			pallet_commitments::Pallet::<Test>::get_commitments(metadata_id);
		assert_eq!(participant_submissions[0], Commitment::new(commit_hash, account.clone()));

		let participant_submission_block =
			Pallet::<Test>::get_commitment_blocks(metadata_id);
		assert_eq!(participant_submission_block.len(), 1);

		assert_eq!(
			participant_submission_block.into_inner()[0],
			(account.into(), System::block_number())
		);
		assert_eq!(
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id),
			None // not enough hashes submitted to start
		)
	})
}

#[test]
fn commit_processing_result_hash_err_if_consensus_has_completed() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	test_externalities.execute_with(|| {
		// a bit more than 2/3 of the participants
		for i in 0..172 {
			let commit_hash = create_commit_hash(test_hash_1, i);
			let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash,
			})
			.encode();
			let signature = test_keys[i as usize].sign(&commit_call);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call,
				signature,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature2 = test_keys[i as usize].sign(&commit_call2);
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call2,
				signature2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		for i in 0..172 {
			let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature = test_keys[i as usize].sign(&reveal_call);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call,
				signature,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature = test_keys[i as usize].sign(&reveal_call);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call,
				signature,
				test_keys[i as usize].public(),
			));
		}
		assert_ok!(Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id));

		let test_key_2 = test_keys[1].clone();
		let late_commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id,
			hash: test_hash_1,
		})
		.encode();
		let late_signature = test_key_2.sign(&late_commit_call);
		assert_err!(
			Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				late_commit_call,
				late_signature,
				test_key_2.public(),
			),
			Error::<Test>::AlreadyProcessedMetadata
		);
	})
}

#[test]
fn commit_processing_result_hash_and_check_tokens_if_consensus_has_completed() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	test_externalities.execute_with(|| {
		let mut winning_accounts = Vec::new();
		let mut non_winning_accounts = Vec::new();

		// a bit more than 2/3 of the participants
		for i in 0..172 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);

			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			non_winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// reveal hash
		for i in 0..172 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		assert_ok!(Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id));
		// check rewards of winning participants
		for (account, free_balance) in winning_accounts {
			let new_balance_after_reward = <Test as Config>::LocalCurrency::free_balance(&account);
			assert!(new_balance_after_reward >= free_balance);
		}

		// check balance of nodes who submitted wrong result
		for (account, free_balance) in non_winning_accounts {
			let new_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			// after burning tokens
			assert_eq!(new_balance, free_balance - FundsToLock::get());
		}
	})
}

#[test]
fn issue_rewards_is_err_if_no_submissions_provided() {
	let (mut test_externalities, _test_accounts) = new_test_ext();
	let metadata_id: MetadataId = 0;
	test_externalities.execute_with(|| {
		assert_eq!(
			Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id),
			Err(Error::<Test>::NoSolutionProvided.into())
		);
	})
}

#[test]
fn consensus_is_completed_if_majority_supply_same_hash() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id: MetadataId = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	test_externalities.execute_with(|| {
		// a bit more than 2/3 of the participants
		for i in 0..172 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// reveal hash
		for i in 0..172 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		assert_ok!(Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id));

		assert_eq!(
			Pallet::<Test>::get_round_state(metadata_id),
			Some(RoundState::Completed)
		);

		// check that the storage got emptied after successful consensus
		assert_eq!(
			Pallet::<Test>::get_commitment_blocks(metadata_id),
			crate::CommittedSubmissions::<Test>::default()
		);
		assert_eq!(
			pallet_commitments::Commits::<Test>::get(metadata_id)
				.iter()
				.collect::<Vec<_>>()
				.len(),
			0
		);
	})
}

#[test]
fn consensus_is_disputed_if_hash_frequency_not_enough() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id: MetadataId = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	// here we submit an equal number of different hashes
	// since there is no clear majority, the consensus will end in a disputed state
	test_externalities.execute_with(|| {
		for i in 0..128 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 128..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// reveal hash
		for i in 0..128 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 128..255 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		// Test the >66% case
		assert_eq!(
			Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id),
			Err(Error::<Test>::ConsensusNotReached.into())
		);

		assert_eq!(
			Pallet::<Test>::get_round_state(metadata_id),
			Some(RoundState::Disputed)
		);

		assert_eq!(Pallet::<Test>::get_commitment_blocks(metadata_id).len(), 255);
		assert_eq!(
			pallet_commitments::Commits::<Test>::get(metadata_id)
				.iter()
				.collect::<Vec<_>>()
				.len(),
			255
		);

		// Test the >50% case
		RoundStates::<Test>::remove(metadata_id);

		Pallet::<Test>::set_majority_type(RawOrigin::Root.into(), Majority::OneHalf).unwrap();

		assert_eq!(
			Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id),
			Ok(())
		);

		assert_eq!(
			Pallet::<Test>::get_round_state(metadata_id),
			Some(RoundState::Completed)
		);

		// check that the storage got emptied after successful consensus
		assert_eq!(Pallet::<Test>::get_commitment_blocks(metadata_id).len(), 0);
		assert!(pallet_commitments::Commits::<Test>::get(metadata_id)
			.iter()
			.collect::<Vec<_>>()
			.is_empty(),);
	})
}

// calling issue reward
#[test]
fn reward_issuance_on_finalization() {
	let (mut test_externalities, test_keys) = new_test_ext();

	let metadata_id: MetadataId = 0;
	let (test_hash_1, ..) = get_hashes();

	test_externalities.execute_with(|| {
		let mut winning_accounts = Vec::new();

		// a bit more than 2/3 of the participants
		for i in 0..172 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// revealing hash
		// correct result
		for i in 0..172 {
			let reveal_call = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature = test_keys[i as usize].sign(&reveal_call);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call,
				signature,
				test_keys[i as usize].public(),
			));
		}

		// updating system block for reward issuence
		System::set_block_number(reveal_window_block + RevealWindowLength::get() as u64);
		<Pallet<Test> as Hooks<<Test as frame_system::Config>::BlockNumber>>::on_finalize(
			System::block_number(),
		);

		assert_eq!(
			Pallet::<Test>::get_round_state(metadata_id),
			Some(RoundState::Completed)
		);
		let reward = Reward::get();
		// check rewards of winning participants
		for (account, free_balance) in winning_accounts {
			let new_balance_after_reward = <Test as Config>::LocalCurrency::free_balance(&account);
			assert_eq!(new_balance_after_reward, free_balance + (reward / 172));
		}
	})
}

#[test]
fn set_majority_type_changes_majority_type() {
	let (mut test_externalities, _) = new_test_ext();

	test_externalities.execute_with(|| {
		let prev_majority_type = MajorityType::<Test>::get();

		Pallet::<Test>::set_majority_type(RawOrigin::Root.into(), Majority::OneHalf).unwrap();

		assert_ne!(MajorityType::<Test>::get(), prev_majority_type);
	})
}

#[test]
fn resolve_metadata_dispute_works() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id: MetadataId = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	test_externalities.execute_with(|| {
		for i in 0..128 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 128..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// reveal hash
		for i in 0..128 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 128..255 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		assert_err!(
			Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id),
			Error::<Test>::ConsensusNotReached
		);

		// Resolve the dispute using original test_hash
		assert_ok!(Pallet::<Test>::resolve_metadata_dispute(
			RawOrigin::Root.into(),
			metadata_id,
			test_hash_1,
		));

		assert_eq!(
			Pallet::<Test>::get_round_state(metadata_id),
			Some(RoundState::ManuallyResolved)
		);
	});
}

#[test]
fn commit_processing_result_hash_if_node_has_insufficient_tokens() {
	let (mut test_externalities, _test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash_1, ..) = get_hashes();

	test_externalities.execute_with(|| {
		let account_with_insufficient_tokens = primitives::shared::Pair::generate().0;
		let commit_hash_1 = create_commit_hash(test_hash_1, 0);
		let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id,
			hash: commit_hash_1,
		})
		.encode();
		let signature_1 = account_with_insufficient_tokens.sign(&commit_call_1);

		assert_err!(
			Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				account_with_insufficient_tokens.public(),
			),
			Error::<Test>::InsufficientBalance
		);
	})
}

#[test]
fn commit_processing_result_hash_err_if_node_has_submitted_twice() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash, ..) = get_hashes();

	test_externalities.execute_with(|| {
		let commit_hash = create_commit_hash(test_hash, 0);
		let commit_call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
			metadata_id,
			hash: commit_hash,
		})
		.encode();
		let signature = test_keys[0].sign(&commit_call);
		assert_ok!(Pallet::<Test>::commit_processing_result_hash(
			RawOrigin::None.into(),
			commit_call.clone(),
			signature.clone(),
			test_keys[0].public(),
		));
		assert_err!(
			Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call,
				signature,
				test_keys[0].public(),
			),
			Error::<Test>::AlreadyCommitted
		);
	})
}

#[test]
fn commit_processing_result_hash_err_if_exceeds_participants_limit() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash_1, ..) = get_hashes();

	test_externalities.execute_with(|| {
		for i in 0..255 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		let commit_hash_past_threshold = create_commit_hash(test_hash_1, 255);
		let commit_call_past_threshold =
			MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_past_threshold,
			})
			.encode();
		let signature_past_threshold = test_keys[255].sign(&commit_call_past_threshold);

		assert_err!(
			Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_past_threshold,
				signature_past_threshold,
				test_keys[255].public(),
			),
			Error::<Test>::SubmissionExceedsMaxParticipantCount
		);
	})
}

#[test]
fn commit_processing_result_hash_with_burn_fund_cases() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	test_externalities.execute_with(|| {
		let mut winning_accounts = Vec::new();
		let mut non_winning_accounts = Vec::new();

		// a bit more than 2/3 of the participants
		for i in 0..172 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 172..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);
			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			non_winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// revealing hash
		// correct result
		for i in 0..172 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		// incorrect result
		for i in 172..200 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);
			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		// incorrect reveal hash and accounts from 220 to 255 didn't provided fulfillment
		for i in 200..220 {
			let reveal_call_invalid = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature = test_keys[i as usize].sign(&reveal_call_invalid);

			assert!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_invalid,
				signature,
				test_keys[i as usize].public(),
			)
			.is_err());
		}

		assert_ok!(Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id));

		// check rewards of winning participants
		for (account, free_balance) in winning_accounts {
			let new_balance_after_reward = <Test as Config>::LocalCurrency::free_balance(&account);
			assert!(new_balance_after_reward >= free_balance);
		}

		// check balance of nodes who submitted wrong result
		for (account, free_balance) in non_winning_accounts {
			let new_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			// after burning tokens
			assert_eq!(new_balance, free_balance - FundsToLock::get());

			// withdrawing all the remaining funds in order to check lock got removed or not
			assert_ok!(<Test as Config>::LocalCurrency::withdraw(
				&account,
				new_balance,
				WithdrawReasons::all(),
				ExistenceRequirement::KeepAlive
			));
		}
	})
}

#[test]
fn process_hash_with_configurable_burn_fund_case() {
	let (mut test_externalities, test_keys) = new_test_ext();
	let metadata_id: MetadataId = 0;
	let (test_hash_1, test_hash_2) = get_hashes();

	// here we submit an equal number of different hashes
	// since there is no clear majority, the consensus will end in a disputed state
	test_externalities.execute_with(|| {
		let mut winning_accounts = Vec::new();
		let mut non_winning_accounts = Vec::new();
		let mut non_winners_without_burning = Vec::new();

		for i in 0..200 {
			let commit_hash_1 = create_commit_hash(test_hash_1, i);
			let commit_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_1,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&commit_call_1);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			if i < 150 {
				winning_accounts.push((account, free_balance));
			} else {
				non_winners_without_burning.push((account, free_balance));
			}

			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}

		for i in 200..255 {
			let commit_hash_2 = create_commit_hash(test_hash_2, i);
			let commit_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash: commit_hash_2,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&commit_call_2);

			let account = get_account_from_public(test_keys[i as usize].public());
			let free_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			non_winning_accounts.push((account, free_balance));
			assert_ok!(Pallet::<Test>::commit_processing_result_hash(
				RawOrigin::None.into(),
				commit_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		let reveal_window_block =
			pallet_commitments::Pallet::<Test>::get_reveal_window_start(metadata_id)
				.unwrap();
		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block);

		// reveal hash
		for i in 0..150 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}
		for i in 200..255 {
			let reveal_call_2 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_2,
				random_seed: i,
			})
			.encode();
			let signature_2 = test_keys[i as usize].sign(&reveal_call_2);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_2,
				signature_2,
				test_keys[i as usize].public(),
			));
		}

		// updating system block for revealing hashes
		System::set_block_number(reveal_window_block + 4);

		for i in 150..200 {
			let reveal_call_1 = MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				reveal_hash: test_hash_1,
				random_seed: i,
			})
			.encode();
			let signature_1 = test_keys[i as usize].sign(&reveal_call_1);

			assert_ok!(Pallet::<Test>::reveal_processing_result_hash(
				RawOrigin::None.into(),
				reveal_call_1,
				signature_1,
				test_keys[i as usize].public(),
			));
		}

		assert_ok!(Pallet::<Test>::issue_rewards(RawOrigin::None.into(), metadata_id));

		// check rewards of winning participants
		for (account, free_balance) in winning_accounts {
			let new_balance_after_reward = <Test as Config>::LocalCurrency::free_balance(&account);
			assert!(new_balance_after_reward >= free_balance);
		}

		// check balance of nodes who submitted wrong result
		for (account, free_balance) in non_winning_accounts {
			let new_balance = <Test as Config>::LocalCurrency::free_balance(&account);
			// after burning tokens
			assert_eq!(new_balance, free_balance - FundsToLock::get());
		}

		// check rewards of winning participants
		for (account, free_balance) in non_winners_without_burning {
			let new_balance = <Test as Config>::LocalCurrency::free_balance(&account);

			assert!(new_balance == free_balance);

			if <Test as Config>::EnforceBurningTokens::get() {
				assert_eq!(new_balance, free_balance - FundsToLock::get());
			}
		}
	})
}
