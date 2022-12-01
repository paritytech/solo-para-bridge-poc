use crate::{mock::*, Commit, Commitment, CommitmentError, Commits, Pallet, Reveal, RevealWindow};

use frame_support::{assert_err, assert_ok, pallet_prelude::*};
use rand::Rng;
use sp_io::hashing::blake2_256;

pub fn create_commit_hash(original_data: H256) -> (H256, u8) {
	let mut rng = rand::thread_rng();
	let random_seed = rng.gen::<u8>();
	let mut combined = original_data.encode();
	combined.push(random_seed);
	let hash = blake2_256(&combined);
	(H256(hash), random_seed)
}

#[test]
fn add_successful_commit() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let account = get_accounts().pop().unwrap();
		let hash = H256::from_low_u64_be(123);
		let commit_key = 1;

		assert_ok!(Commitments::commit(account.clone(), hash, commit_key));
		assert_eq!(Commitments::get_commitments(commit_key)[0], Commitment::new(hash, account));
	})
}

#[test]
fn reject_duplicate_reveal() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let mut accounts = get_accounts();
		let alice = accounts.pop().unwrap();
		let original_hash = H256::from_low_u64_be(456);
		let commit_key = 1;

		let (commit_hash, random_seed) = create_commit_hash(original_hash);
		let mut commitment = Commitment::new(commit_hash, alice.clone());

		RevealWindow::<Test>::insert(commit_key, 0);
		Commits::<Test>::try_mutate(commit_key, |commitments| {
			commitments.try_push(commitment.clone())
		})
		.unwrap();

		assert_ok!(Commitments::reveal(alice.clone(), original_hash, commit_key, random_seed));
		commitment.fulfill(original_hash);
		commitment.set_reveal_status();
		assert_eq!(Commitments::get_commitments(commit_key)[0], commitment);
		assert_eq!(
			Commitments::reveal(alice, original_hash, commit_key, random_seed),
			Err(CommitmentError::AlreadyRevealed)
		);
	})
}

#[test]
fn add_successful_reveal() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let mut accounts = get_accounts();
		let alice = accounts.pop().unwrap();
		let original_hash = H256::from_low_u64_be(123);
		let commit_key = 1;

		let (commit_hash, random_seed) = create_commit_hash(original_hash);
		let mut commitment = Commitment::new(commit_hash, alice.clone());

		RevealWindow::<Test>::insert(commit_key, 0);
		Commits::<Test>::try_mutate(commit_key, |commitments| {
			commitments.try_push(commitment.clone())
		})
		.unwrap();

		assert_ok!(Commitments::reveal(alice.clone(), original_hash, commit_key, random_seed));
		commitment.fulfill(original_hash);
		commitment.set_reveal_status();
		assert_eq!(Commitments::get_commitments(commit_key)[0], commitment);
	})
}

#[test]
fn reveal_fails_if_answer_is_incorrect() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let mut accounts = get_accounts();
		let alice = accounts.pop().unwrap();
		let original_hash = H256::from_low_u64_be(123);
		let incorrect_hash = H256::from_low_u64_be(456);
		let commit_key = 1;
		let (commit_hash, random_seed) = create_commit_hash(incorrect_hash);

		RevealWindow::<Test>::insert(commit_key, 0);
		Commits::<Test>::try_mutate(commit_key, |commitments| {
			commitments.try_push(Commitment::new(commit_hash, alice.clone()))
		})
		.unwrap();

		assert_err!(
			Commitments::reveal(alice.clone(), original_hash, commit_key, random_seed),
			CommitmentError::IncorrectRevealedHash
		);
	})
}

#[test]
fn reveal_period_false_if_not_revealed_inside_period() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let mut accounts = get_accounts();
		let alice = accounts.pop().unwrap();
		let original_hash = H256::from_low_u64_be(123);
		let commit_key = 1;
		let (commit_hash, random_seed) = create_commit_hash(original_hash);
		let mut commitment = Commitment::new(commit_hash, alice.clone());
		Commits::<Test>::try_mutate(commit_key, |commitments| {
			commitments.try_push(commitment.clone())
		})
		.unwrap();

		// No Reveal window set
		assert_ok!(Commitments::reveal(alice.clone(), original_hash, commit_key, random_seed));
		commitment.fulfill(original_hash);
		assert_eq!(Commitments::get_commitments(commit_key)[0], commitment);
	})
}

#[test]
fn reveal_fails_if_no_commitment_made() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let mut accounts = get_accounts();
		let alice = accounts.pop().unwrap();
		let original_hash = H256::from_low_u64_be(123);
		let commit_key = 1;
		let mut rng = rand::thread_rng();
		let random_seed = rng.gen::<u8>();

		RevealWindow::<Test>::insert(commit_key, 0);

		assert_err!(
			Commitments::reveal(alice, original_hash, commit_key, random_seed),
			CommitmentError::NoCommitmentFound
		);
	})
}

#[test]
fn recreate_commit_hash_creates_commit() {
	let mut test_externalities = new_test_ext();
	test_externalities.execute_with(|| {
		let original_hash = H256::from_low_u64_be(123);
		let (original_commit_hash, random_seed) = create_commit_hash(original_hash);
		let recreated_original = Pallet::<Test>::recreate_commit_hash(original_hash, random_seed);
		assert_eq!(original_commit_hash, recreated_original);
	})
}
