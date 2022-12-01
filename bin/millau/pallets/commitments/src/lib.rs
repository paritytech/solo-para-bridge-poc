#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, PalletError};
	use sp_core::H256;

	use sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps, SaturatedConversion};

	use sp_std::{fmt::Debug, prelude::*};

	// A unique identifier of the current subject that commit/reveals are centered on
	type CommitKey = u64;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		// Amount of blocks in which reveals of a given key are allowed, following a commit window
		// for the same key
		type RevealWindowLength: Get<u8>;
		#[pallet::constant]
		type MaxParticipants: Get<u32>;
		type Hash: From<H256>
			+ Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ Debug
			+ MaybeDisplay
			+ SimpleBitOps
			+ Ord
			+ Default
			+ Copy
			+ CheckEqual
			+ sp_std::hash::Hash
			+ AsRef<[u8]>
			+ AsMut<[u8]>
			+ MaxEncodedLen;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	// should we need to add block_num here for issue rewards?
	#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, Copy)]
	#[codec(mel_bound())]
	/// A committed answer towards some topic which can be fulfilled at a later point in time.
	/// Unique to participant and the commit/reveal round in which it was created
	pub struct Commitment<Hash, AccountId> {
		// The commitment hash that established the Commitment
		commit: Hash,
		// The revealed "answer" that fulfills the Commitment
		fulfillment: Option<Hash>,
		// account id
		submitter: AccountId,
		// was fulfilment submitted in reveal period
		in_reveal_period: bool,
	}

	impl<Hash, AccountId> Commitment<Hash, AccountId> {
		pub fn new(commit: Hash, submitter: AccountId) -> Self {
			// It's expected that a commitment is created with only the commit hash
			Commitment { commit, fulfillment: None, submitter, in_reveal_period: false }
		}

		// Fulfill an existing Commitment by providing the answer that was used to create
		// the `commit`
		pub fn fulfill(&mut self, fulfillment: Hash) {
			self.fulfillment = Some(fulfillment);
		}

		pub fn get_fulfillment(&self) -> Option<&Hash> {
			self.fulfillment.as_ref()
		}

		pub fn get_submitter(&self) -> &AccountId {
			&self.submitter
		}

		pub fn set_reveal_status(&mut self) {
			self.in_reveal_period = true;
		}

		pub fn was_in_reveal_period(&self) -> bool {
			self.in_reveal_period
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn get_commitments)]
	/// Track commitments made by committing accounts, and commitment_key
	pub type Commits<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		CommitKey,
		BoundedVec<Commitment<<T as pallet::Config<I>>::Hash, T::AccountId>, T::MaxParticipants>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_reveal_window_start)]
	/// The window of blocks for which a given subject's reveal window is scheduled.
	pub type RevealWindow<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		CommitKey,
		u64,
		OptionQuery,
	>;

	#[derive(Encode, Decode, TypeInfo, PalletError, Debug, PartialEq)]
	pub enum CommitmentError {
		/// Attempted to reveal a hash that did not match the original contribution
		IncorrectRevealedHash,
		/// Attempted to reveal a hash for a commitment that was not found for a given account
		NoCommitmentFound,
		/// If the commitment was revealed once, we don't allow for further reveals.
		AlreadyRevealed,
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		pub fn is_in_reveal_window(commit_key: CommitKey) -> bool {
			if let Some(window_start) = Self::get_reveal_window_start(commit_key) {
				let window_end = window_start + T::RevealWindowLength::get() as u64;
				let current_block = <frame_system::Pallet<T>>::block_number().saturated_into::<u64>();

				window_start <= current_block && current_block <= window_end
			} else {
				false
			}
		}

		pub fn get_reveal_window(
			commit_key: CommitKey,
		) -> Option<(u64, u64)> {
			Self::get_reveal_window_start(commit_key)
				.map(|start| (start, start + T::RevealWindowLength::get() as u64))
		}
	}

	pub trait Commit<AccountId, Hash> {
		fn commit(
			committer: AccountId,
			commit_hash: Hash,
			commit_key: CommitKey,
		) -> Result<(), CommitmentError>;
	}

	pub trait Reveal<AccountId, Hash>
	where
		Hash: From<sp_core::H256>,
	{
		fn recreate_commit_hash(answer: Hash, random_seed: u8) -> sp_core::H256;
		fn reveal(
			committer: AccountId,
			reveal_hash: Hash,
			commit_key: CommitKey,
			random_seed: u8,
		) -> Result<(), CommitmentError>;
	}

	impl<T: Config<I>, I: 'static> Commit<T::AccountId, <T as pallet::Config<I>>::Hash>
		for Pallet<T, I>
	{
		fn commit(
			who: T::AccountId,
			commit_hash: <T as pallet::Config<I>>::Hash,
			commit_key: CommitKey,
		) -> Result<(), CommitmentError> {
			// Basic checks expected in wrapper function
			Commits::<T, I>::try_mutate(commit_key, |commitments| {
				commitments
					.try_push(Commitment::new(commit_hash, who))
					.map_err(|_| CommitmentError::NoCommitmentFound)
			})
		}
	}

	impl<T: Config<I>, I: 'static> Reveal<T::AccountId, <T as pallet::Config<I>>::Hash> for Pallet<T, I>
	where
		<T as pallet::Config<I>>::Hash: From<sp_core::H256>,
	{
		// Attempt to recreate the commitment hash through the process that committers
		// are expected to follow in creating their commitment hashes: hash(reveal_hash +
		// random_seed)
		fn recreate_commit_hash(
			original_hash: <T as pallet::Config<I>>::Hash,
			random_seed: u8,
		) -> sp_core::H256 {
			let mut combined = original_hash.encode();
			combined.push(random_seed);
			let hash = sp_io::hashing::blake2_256(&combined);
			sp_core::H256(hash)
		}

		/// Attempt to "reveal" an answer from the POV of the original committer, by
		/// recreating their original commit message using the random seed that was
		/// originally used by the committer to do the same
		fn reveal(
			who: T::AccountId,
			reveal_hash: <T as pallet::Config<I>>::Hash,
			commit_key: CommitKey,
			random_seed: u8,
		) -> Result<(), CommitmentError> {
			Commits::<T, I>::try_mutate(commit_key, |commitment| {
				for commit in commitment.iter_mut() {
					if commit.submitter == who {
						ensure!(commit.fulfillment.is_none(), CommitmentError::AlreadyRevealed);
						let recreated_commit = Self::recreate_commit_hash(reveal_hash, random_seed);
						ensure!(
							commit.commit == recreated_commit.into(),
							CommitmentError::IncorrectRevealedHash
						);
						commit.fulfill(reveal_hash);
						if Self::is_in_reveal_window(commit_key) {
							commit.set_reveal_status();
						}
						return Ok(())
					}
				}
				Err(CommitmentError::NoCommitmentFound)
			})
		}
	}
}
