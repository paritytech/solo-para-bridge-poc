#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

use frame_support::{
	fail,
	traits::{Currency, LockableCurrency, WithdrawReasons},
};
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

pub use primitives::shared::{Hash, LogicProviderCall, MapToCall, MetadataId, Public};
use sp_core::crypto::AccountId32;
pub use sp_runtime::RuntimeAppPublic;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::{DispatchResult, RawOrigin},
		pallet_prelude::*,
		traits::ExistenceRequirement,
	};
	use primitives::shared::Signature;
	use sp_std::{fmt::Debug, prelude::*};

	use frame_system::pallet_prelude::*;
	use itertools::Itertools;
	use num_rational::Ratio;
	use pallet_commitments::{Commit, CommitmentError, Reveal};
	use sp_runtime::traits::CheckedSub;
	use sp_std::vec::Vec;

	pub type BalanceOf<T> =
		<<T as Config>::LocalCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub trait TemplateBridgedXcm<T: Config> {
		fn send_transact(
			origin: OriginFor<T>,
			proof: Vec<u8>,
			delivery_and_dispatch_fee: u64,
		) -> Result<([u8; 32], xcm::v3::MultiAssets), xcm::v3::SendError>;
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_commitments::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		// Constants
		#[pallet::constant]
		type MaxCallPayloadLength: Get<u16>;
		#[pallet::constant]
		type EnforceBurningTokens: Get<bool>;
		#[pallet::constant]
		type Reward: Get<BalanceOf<Self>>;
		#[pallet::constant]
		type FundsToLock: Get<BalanceOf<Self>>;
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		type WeightInfo: WeightInfo;
		// Outer types
		type LocalCurrency: Currency<<Self as frame_system::Config>::AccountId>
			+ LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		type Bridging: crate::TemplateBridgedXcm<Self>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	// Run per metadata / metadata ids
	#[derive(Debug, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[codec(mel_bound())]
	pub enum RoundState {
		Completed,
		Disputed,
		ManuallyResolved,
	}

	/// The majority type
	#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[codec(mel_bound())]
	pub enum Majority {
		TwoThirds,
		OneHalf,
	}

	impl Default for Majority {
		fn default() -> Self {
			Majority::TwoThirds
		}
	}

	impl Majority {
		pub fn to_ratio(self) -> Ratio<u32> {
			match self {
				Majority::OneHalf => Ratio::new_raw(50u32, 100u32),
				Majority::TwoThirds => Ratio::new_raw(66u32, 100u32),
			}
		}
	}

	impl<T> From<CommitmentError> for Error<T> {
		fn from(error: CommitmentError) -> Self {
			match error {
				CommitmentError::IncorrectRevealedHash => Error::<T>::IncorrectRevealedHash,
				CommitmentError::NoCommitmentFound => Error::<T>::NoCommitmentFound,
				CommitmentError::AlreadyRevealed => Error::<T>::AlreadyRevealed,
			}
		}
	}

	pub type CommittedSubmissions<T> = BoundedVec<
		(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::BlockNumber),
		<T as pallet_commitments::Config>::MaxParticipants,
	>;

	/// The storage pertains to store the block number of each submission.
	#[pallet::storage]
	#[pallet::getter(fn get_commitment_blocks)]
	pub(super) type CommitmentBlockNumbers<T: Config> =
		StorageMap<_, Blake2_128Concat, MetadataId, CommittedSubmissions<T>, ValueQuery>;

	#[pallet::storage]
	pub(super) type MajorityType<T: Config> = StorageValue<_, Majority, ValueQuery>;

	/// The storage element that tracks the result of executed consensus for each metadata.
	#[pallet::storage]
	#[pallet::getter(fn get_round_state)]
	pub(super) type RoundStates<T: Config> =
		StorageMap<_, Blake2_128Concat, MetadataId, RoundState>;

	#[pallet::storage]
	#[pallet::getter(fn get_processed_hashes)]
	pub(super) type ProcessedHashes<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		MetadataId,
		<T as pallet_commitments::Config>::Hash,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		HashCommitted(<T as pallet_commitments::Config>::Hash),
		HashRevealed(<T as pallet_commitments::Config>::Hash),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		SubmissionExceedsMaxParticipantCount,
		/// This error variant is returned when something goes wrong when
		/// determining what hash was correct, e.g. failed to find the most frequently
		/// occurring hash.
		ConsensusError,
		ConsensusNotReached,
		NoSolutionProvided,
		/// Attempt to do certain action which is not permitted in the current [`RoundState`].
		IllegalState,
		// Key given for this node could not be decoded
		InvalidGivenPublicKey,
		InsufficientBalance,
		// Participant already made a commitment for the given commit key
		AlreadyCommitted,
		// Attempted to submit result for processed Metadata Id
		AlreadyProcessedMetadata,
		// Participant attempted to commit an answer during the reveal period - when answers could
		// have been revealed
		AttemptedCommitInRevealPeriod,
		/// Attempted to reveal a hash that did not match the original contribution
		IncorrectRevealedHash,
		/// Attempted to reveal a hash for a commitment that was not found for a given account
		NoCommitmentFound,
		/// Attempted to reveal a commit outside of the reveal window
		AttemptedRevealOutsideWindow,
		/// When the commitment was already revealed
		AlreadyRevealed,
		/// A Call payload sent from the offchain component could not be interpreted as a call with
		/// the intended arguments
		InvalidCallPayload,
		/// Incoming vector containing payload was larger than expected
		EncodedCallTooLarge,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// During finalization, check what metadatas are ready
		// to be scheduled & schedule them.
		fn on_finalize(_current_block: BlockNumberFor<T>) {
			for (metadata_id, starting_block) in pallet_commitments::RevealWindow::<T>::iter() {
				if <frame_system::Pallet<T>>::block_number() >=
					starting_block + T::RevealWindowLength::get().into()
				{
					if let Err(error) = Pallet::<T>::issue_rewards(
						RawOrigin::None.into(),
						metadata_id,
					) {
						log::error!(target: "runtime::template", "Consensus for metadata {} has failed. ({:?})", metadata_id, error);
						// Consensus not reached.
						RoundStates::<T>::insert(metadata_id, RoundState::Disputed);
					}

					pallet_commitments::RevealWindow::<T>::remove(metadata_id);
				}
			}
		}

		// Return the weight consumed in on_finalize to make sure
		// that the block does not get overweight
		// due to the computations in on_finalize
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<T as Config>::WeightInfo::on_finalize(
				pallet_commitments::RevealWindow::<T>::iter_keys().count() as u32,
				T::MaxParticipants::get(),
			)
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Commit the resulting hash wrapped with random seed of metadata processing.
		///
		/// # Parameters
		/// * `metadata_id` - the metadatar id
		/// * `hash` - the hashed result (resultant hash + random seed)
		/// * `public` - the public key of the sender
		///
		/// # Errors
		/// Returns a `SubmissionExceedsMaxParticipantCount` error when this submission exceeds the
		/// maximum number of consensus participants.
		/// Returns a `AlreadyCommitted` error when this submission is duplicate.
		#[pallet::weight(<T as Config>::WeightInfo::commit_processing_result_hash())]
		pub fn commit_processing_result_hash(
			origin: OriginFor<T>,
			payload: Vec<u8>,
			_signature: Signature,
			public: Public,
		) -> DispatchResult {
			ensure_none(origin)?;
			let who = Self::to_account_id(public)?;
			ensure!(
				payload.len() < T::MaxCallPayloadLength::get().into(),
				Error::<T>::EncodedCallTooLarge
			);
			let decoded_call =
				MapToCall::decode(&mut &payload[..]).map_err(|_| Error::<T>::InvalidCallPayload)?;

			if let MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
				metadata_id,
				hash,
			}) = decoded_call
			{
				ensure!(
					Self::get_commitment_blocks(metadata_id).len() <
						T::MaxParticipants::get() as usize,
					Error::<T>::SubmissionExceedsMaxParticipantCount
				);

				ensure!(
					!pallet_commitments::Pallet::<T>::get_commitments(metadata_id)
						.into_iter()
						.any(|commitment| commitment.get_submitter() == &who),
					Error::<T>::AlreadyCommitted
				);
				ensure!(
					!ProcessedHashes::<T>::contains_key(metadata_id),
					Error::<T>::AlreadyProcessedMetadata
				);
				ensure!(
					!pallet_commitments::Pallet::<T>::is_in_reveal_window(metadata_id),
					Error::<T>::AttemptedCommitInRevealPeriod
				);

				let new_balance = T::LocalCurrency::free_balance(&who)
					.checked_sub(&T::FundsToLock::get())
					.ok_or(Error::<T>::InsufficientBalance)?;

				// check for balance is available to lock or not
				T::LocalCurrency::ensure_can_withdraw(
					&who,
					T::FundsToLock::get(),
					WithdrawReasons::all(),
					new_balance,
				)?;

				Self::do_commit_processing_result_hash(
					metadata_id,
					hash.into(),
					who.clone(),
				)?;

				// Lock afterwards to ensure that lock only happens after checks in commit
				// pallet
				Self::lock_tokens(metadata_id, &who, T::FundsToLock::get())
			} else {
				fail!(Error::<T>::InvalidCallPayload);
			}
		}

		/// Reveal the resulting hash of metadata processing.
		///
		/// # Parameters
		/// * `original_hash` - the original hash
		/// * `random_seed` - any random seed which was use at the time of commitment
		/// * `metadata_id` - the metadatar id
		/// * `public` - the public key of the sender
		///
		/// # Errors
		/// Returns a `CommitmentError` error generated from commitment pallet.
		/// Returns a `InvalidGivenPublicKey` error when the provided public key is invalid
		#[pallet::weight(<T as Config>::WeightInfo::reveal_processing_result_hash())]
		pub fn reveal_processing_result_hash(
			origin: OriginFor<T>,
			payload: Vec<u8>,
			_signature: Signature,
			public: Public,
		) -> DispatchResult {
			ensure_none(origin.clone())?;
			let who = Self::to_account_id(public)?;
			ensure!(
				payload.len() < T::MaxCallPayloadLength::get().into(),
				Error::<T>::EncodedCallTooLarge
			);
			let decoded_call =
				MapToCall::decode(&mut &payload[..]).map_err(|_| Error::<T>::InvalidCallPayload)?;
			if let MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
				metadata_id,
				random_seed,
				reveal_hash,
				..
			}) = decoded_call
			{
				let reveal_result = pallet_commitments::Pallet::<T>::reveal(
					who.clone(),
					reveal_hash.into(),
					metadata_id,
					random_seed,
				);
				if reveal_result.is_err() {
					if reveal_result.as_ref().unwrap_err() ==
						&CommitmentError::IncorrectRevealedHash
					{
						Self::release_tokens(metadata_id, &who);
						Self::burn_tokens(&who, T::FundsToLock::get());
						CommitmentBlockNumbers::<T>::try_mutate::<
							MetadataId,
							(),
							Error<T>,
							_,
						>(metadata_id, |committed_submission| {
							let index = committed_submission
								.iter()
								.position(|element| element.0 == who)
								.ok_or(Error::<T>::NoCommitmentFound)?;
							committed_submission.remove(index);
							Ok(())
						})?;
						pallet_commitments::Commits::<T>::try_mutate::<
							MetadataId,
							(),
							Error<T>,
							_,
						>(metadata_id, |committed_submission| {
							let index = committed_submission
								.iter()
								.position(|element| element.get_submitter() == &who)
								.ok_or(Error::<T>::NoCommitmentFound)?;
							committed_submission.remove(index);
							Ok(())
						})?;
					}
					fail!(Error::<T>::from(reveal_result.unwrap_err()));
				}
				Self::deposit_event(Event::HashRevealed(reveal_hash.into()));

				Ok(())
			} else {
				fail!(Error::<T>::InvalidCallPayload)
			}
		}

		/// Run the consensus & issue the rewards to the quickest submitters of the correct
		/// solution. As the least unit of time on blockchain is a block, we evenly divide the
		/// designated reward between **all** participants, who submitted the correct solution in
		/// the same block. If a node submits the correct solution in the following block, this node
		/// will *not* get any reward.
		/// The correct solution in the hash which occurs in more that majority of the submissions
		/// (e.g. 67 out of 100 nodes submit the same hash, regardless of the block number, or >50%
		/// - that's configured using the [`MajorityType`]).
		///
		/// # Parameters
		/// * `metadata_id` - the metadata which reached the target block_number for
		///   consensus.
		///
		/// # Errors
		/// Returns a `ConsensusError` when the extrinsic fails to compute the most frequent hash,
		/// or any other unexpected error. **This is an internal error and requires more insight
		/// into the code to determine what happened exactly.**
		#[pallet::weight(<T as Config>::WeightInfo::issue_rewards_to_some_participants(T::MaxParticipants::get())
		.max(<T as Config>::WeightInfo::issue_rewards_to_all_participants(T::MaxParticipants::get())))]
		pub fn issue_rewards(
			origin: OriginFor<T>,
			metadata_id: MetadataId,
		) -> DispatchResult {
			ensure_none(origin.clone())?;
			match Self::calculate_rewards(metadata_id, None) {
				Ok((winners, reward)) => {
					Self::do_issue_rewards(&winners, reward);
					RoundStates::<T>::insert(metadata_id, RoundState::Completed);

					let winning_hash = pallet_commitments::Commits::<T>::get(metadata_id)
						.into_iter()
						.find(|commitment| commitment.get_submitter() == &winners[0])
						.map(|commitment| {
							commitment.get_fulfillment().map(ToOwned::to_owned).unwrap()
						})
						.ok_or(Error::<T>::ConsensusError)?;

					Self::release_tokens_of_participants(metadata_id)?;
					Self::burn_eligible_account_tokens(&winning_hash, metadata_id)?;

					// cleaning up storage
					pallet_commitments::Commits::<T>::remove(metadata_id);
					CommitmentBlockNumbers::<T>::remove(metadata_id);
					ProcessedHashes::<T>::insert(metadata_id, winning_hash);

					// Consider moving out into a shared crate. What we end up sending should
					// probably end up being something that can prove what happened at the pallet
					// level here, but probably not know too much about this business logic
					let proof = (metadata_id, winning_hash).encode();

					let _xcm_send = T::Bridging::send_transact(
						origin, proof, // Enough gas
						4259640838,
					);

					Ok(())
				},
				Err(err) => {
					// Consensus not reached.
					//RoundStates::<T>::insert(metadata_id, RoundState::Disputed);
					Err(err.into())
				},
			}
		}

		/// Set majority type.
		/// Use this extrinsic to set majority type in runtime.
		/// # Parameters
		/// * `majority_type` - the [majority type][`Majority`] to set.
		#[pallet::weight(<T as Config>::WeightInfo::set_majority_type())]
		pub fn set_majority_type(origin: OriginFor<T>, majority_type: Majority) -> DispatchResult {
			let _ = T::ForceOrigin::ensure_origin(origin)?;

			MajorityType::<T>::put(majority_type);

			Ok(())
		}

		/// Resolve a dispute for some metadata.
		/// Origin must have permissions of the [`ForceOrigin`][Config::ForceOrigin].
		#[pallet::weight(
		<T as Config>::WeightInfo::resolve_metadata_dispute(
		T::MaxParticipants::get()
		  )
		)]
		pub fn resolve_metadata_dispute(
			origin: OriginFor<T>,
			metadata_id: MetadataId,
			force_hash: <T as pallet_commitments::Config>::Hash,
		) -> DispatchResult {
			let _ = T::ForceOrigin::ensure_origin(origin)?;
			ensure!(
				RoundStates::<T>::get(metadata_id) == Some(RoundState::Disputed),
				Error::<T>::IllegalState
			);
			match Self::calculate_rewards(metadata_id, Some(force_hash)) {
				Ok((winners, reward)) => {
					Self::do_issue_rewards(&winners, reward);
					RoundStates::<T>::insert(metadata_id, RoundState::ManuallyResolved);

					Self::release_tokens_of_participants(metadata_id)?;

					Self::burn_eligible_account_tokens(&force_hash, metadata_id)?;

					// cleaning up storage
					pallet_commitments::Commits::<T>::remove(metadata_id);
					CommitmentBlockNumbers::<T>::remove(metadata_id);

					CommitmentBlockNumbers::<T>::remove(metadata_id);

					Ok(())
				},
				Err(e) => Err(e.into()),
			}
		}

		#[pallet::weight(10000000)]
		/// Some call that muse be defined and interpreted(decoded on the target chain)
		pub fn target_chain_call(origin: OriginFor<T>, _val_1: u8, _val_2: u8) -> DispatchResult {
			let _ = ensure_signed(origin);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn do_commit_processing_result_hash(
			metadata_id: MetadataId,
			hash: <T as pallet_commitments::Config>::Hash,
			submitter: T::AccountId,
		) -> DispatchResult {
			let state = RoundStates::<T>::get(metadata_id);
			ensure!(state.is_none(), Error::<T>::IllegalState);

			match pallet_commitments::Pallet::<T>::commit(
				submitter.clone(),
				hash,
				metadata_id,
			) {
				Ok(_) => {
					let current_block = <frame_system::Pallet<T>>::block_number();

					CommitmentBlockNumbers::<T>::try_mutate(
						metadata_id,
						|committed_submission| {
							committed_submission
								.try_push((submitter, current_block))
								.map_err(|_| Error::<T>::SubmissionExceedsMaxParticipantCount)
						},
					)?;
					let commitment_count =
						CommitmentBlockNumbers::<T>::get(metadata_id).len() as u32;
					let commit_ratio =
						Ratio::<u32>::new_raw(commitment_count as u32, T::MaxParticipants::get());

					// If the amount of committers comprises a majority of available committers
					// and we have not yet set a scheduled "reveal" window for this new key
					if commit_ratio > MajorityType::<T>::get().to_ratio() &&
						pallet_commitments::Pallet::<T>::get_reveal_window_start(
							metadata_id,
						)
						.is_none()
					{
						let reveal_window_starting_block =
							current_block + T::RevealWindowLength::get().into();

						// Designate the first block of the reveal period, where participants can
						// begin revealing answers
						pallet_commitments::RevealWindow::<T>::insert(
							metadata_id,
							reveal_window_starting_block,
						);
					}
					Self::deposit_event(Event::HashCommitted(hash));
					Ok(())
				},
				Err(error) => fail!(Error::<T>::from(error)),
			}
		}

		#[allow(clippy::type_complexity)]
		pub fn calculate_rewards(
			metadata_id: MetadataId,
			force_correct_result: Option<<T as pallet_commitments::Config>::Hash>,
		) -> Result<(Vec<T::AccountId>, BalanceOf<T>), Error<T>> {
			// AccountId and their block_num when they submits the result in the form of
			// CommittedSubmissions.
			let committed_blocks = CommitmentBlockNumbers::<T>::get(metadata_id);

			ensure!(!committed_blocks.is_empty(), Error::<T>::NoSolutionProvided);

			let final_submissions_vec =
				pallet_commitments::Pallet::<T>::get_commitments(metadata_id)
					.into_iter()
					.filter(|commitment| {
						commitment.get_fulfillment().is_some() && commitment.was_in_reveal_period()
					})
					.map(|commitment| {
						(commitment.get_submitter().clone(), *commitment.get_fulfillment().unwrap())
					})
					.collect::<Vec<_>>();

			ensure!(!final_submissions_vec.is_empty(), Error::<T>::ConsensusNotReached);

			// get most frequent hashes
			let most_frequent_hash =
				Self::get_most_frequent_hash(&final_submissions_vec, force_correct_result)?;

			// Arrange submissions as per block number
			let mut submissions_with_correct_order =
				committed_blocks.into_iter().sorted_by_key(|(_acc, block)| *block);

			// Figures out the correct submissions by assuming the correct hash is most frequent one
			let correct_submissions = final_submissions_vec
				.into_iter()
				.filter(|submission| submission.1 == most_frequent_hash);

			// list of account_ids who submitted correct result
			let accounts_with_correct_submission =
				&correct_submissions.map(|submissions| submissions.0).collect::<Vec<_>>();

			// the first node who submitted the correct result
			let first_correct_submission = submissions_with_correct_order
				.find(|submission| accounts_with_correct_submission.contains(&submission.0))
				.ok_or(Error::<T>::ConsensusError)?;

			// the first block_num where the correct result has submitted
			let first_correct_block_num = first_correct_submission.1;

			// Account_ids and their block number who are eligible for reward
			let submissions_to_reward =
				submissions_with_correct_order.into_iter().filter(|submission| {
					accounts_with_correct_submission.contains(&submission.0) &&
						submission.1 == first_correct_block_num
				});

			// list of particiapnts who will be rewared
			let participants_to_reward = submissions_to_reward
				.collect::<Vec<_>>()
				.into_iter()
				.chain(sp_std::iter::once(first_correct_submission))
				.map(|item| item.0)
				.collect::<Vec<<T as frame_system::Config>::AccountId>>();

			// amount which will be rewarded to winning participants
			let reward_for_each_participant =
				T::Reward::get() / (participants_to_reward.len() as u32).into();
			Ok((participants_to_reward, reward_for_each_participant))
		}

		fn get_most_frequent_hash(
			submissions_vec: &[(T::AccountId, <T as pallet_commitments::Config>::Hash)],
			force_correct_result: Option<<T as pallet_commitments::Config>::Hash>,
		) -> Result<<T as pallet_commitments::Config>::Hash, Error<T>> {
			if let Some(correct_result) = force_correct_result {
				return Ok(correct_result)
			}
			let mut hash_counts = sp_std::collections::btree_map::BTreeMap::<
				<T as pallet_commitments::Config>::Hash,
				u16,
			>::new();
			for hash in submissions_vec.iter().map(|submission| submission.1) {
				*hash_counts.entry(hash).or_default() += 1;
			}
			let different_hash_count = submissions_vec.len();

			let (most_frequent_hash, occurrences) = hash_counts
				.into_iter()
				.max_by_key(|item| item.1)
				.ok_or(Error::<T>::ConsensusError)?;

			// Normally we wouldn't get such an error, as each hash in the map has occurred at
			// least once.
			let correctness_percentage =
				Ratio::<u32>::new_raw(occurrences as u32, different_hash_count as u32);
			ensure!(
				correctness_percentage > MajorityType::<T>::get().to_ratio(),
				Error::<T>::ConsensusNotReached
			);

			Ok(most_frequent_hash)
		}

		pub fn do_issue_rewards(accounts: &[T::AccountId], reward: BalanceOf<T>) {
			for account in accounts.iter() {
				let imbalance = T::LocalCurrency::issue(reward);
				#[cfg(not(feature = "runtime-benchmarks"))]
				log::info!(target: "runtime::logic-provider", "Issued {:?} tokens", reward);

				T::LocalCurrency::resolve_creating(account, imbalance);
				#[cfg(not(feature = "runtime-benchmarks"))]
				log::info!(target: "runtime::logic-provider", "Resolved {:?} tokens to {:?}", reward, &account);
			}
		}

		pub fn to_account_id(public: Public) -> Result<T::AccountId, Error<T>> {
			let public: sp_core::sr25519::Public = public.into();
			let account_id_32: AccountId32 = public.into();
			if let Ok(account_id) = T::AccountId::decode(&mut &account_id_32.encode()[..]) {
				Ok(account_id)
			} else {
				Err(Error::<T>::InvalidGivenPublicKey)
			}
		}

		pub fn burn_eligible_account_tokens(
			correct_hash: &<T as pallet_commitments::Config>::Hash,
			metadata_id: MetadataId,
		) -> Result<(), Error<T>> {
			let eligible_participants =
				pallet_commitments::Pallet::<T>::get_commitments(metadata_id)
					.iter()
					.filter(|&commitment| {
						commitment.clone().get_fulfillment() != Some(correct_hash) ||
							(T::EnforceBurningTokens::get() &&
								!commitment.clone().was_in_reveal_period())
					})
					.map(|commitment| commitment.get_submitter().clone())
					.collect::<Vec<_>>();
			for participant in eligible_participants {
				Self::burn_tokens(&participant, T::FundsToLock::get());
			}
			Ok(())
		}

		// Release funds of accounts who has submitted the correct result but not in first correct
		// block
		pub fn release_tokens_of_participants(
			metadata_id: MetadataId,
		) -> Result<(), Error<T>> {
			let participants =
				pallet_commitments::Pallet::<T>::get_commitments(metadata_id)
					.iter()
					.map(|commitment| commitment.get_submitter().clone())
					.collect::<Vec<_>>();

			for participant in participants {
				Self::release_tokens(metadata_id, &participant);
			}
			Ok(())
		}

		pub fn lock_tokens(
			metadata_id: MetadataId,
			submitter: &T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			#[cfg(not(feature = "runtime-benchmarks"))]
			log::info!(target: "runtime::logic-provider", "Locking {:?} tokens of {:?}", amount, &submitter);
			T::LocalCurrency::set_lock(
				metadata_id.to_le_bytes(),
				submitter,
				amount,
				WithdrawReasons::all(),
			);
			Ok(())
		}

		pub fn release_tokens(
			metadata_id: MetadataId,
			submitter: &T::AccountId,
		) {
			#[cfg(not(feature = "runtime-benchmarks"))]
			log::info!(target: "runtime::logic-provider", "Releasing tokens of {:?}", &submitter);
			T::LocalCurrency::remove_lock(metadata_id.to_le_bytes(), submitter);
		}

		pub fn burn_tokens(account: &T::AccountId, amount: BalanceOf<T>) {
			let imbalance = T::LocalCurrency::burn(amount);

			if T::LocalCurrency::settle(
				account,
				imbalance,
				WithdrawReasons::all(),
				ExistenceRequirement::KeepAlive,
			)
			.is_ok()
			{
				#[cfg(not(feature = "runtime-benchmarks"))]
				log::info!(target: "runtime::logic-provider", "Burning {:?} tokens of {:?}", amount, &account);
			} else {
				#[cfg(not(feature = "runtime-benchmarks"))]
				log::info!(target: "runtime::logic-provider", "Unable to burn {:?} tokens of {:?}", amount, &account);
			}
		}

		// Specific checks for the unique calls expected by the logic provider pallet
		fn verify_call_public_key_commit(
			public: &Public,
			commit_hash: &<T as pallet_commitments::Config>::Hash,
			mapped_call: &[u8],
			signature: &Signature,
		) -> TransactionValidity {
			match Pallet::<T>::to_account_id(public.clone()) {
				Ok(_account) if public.verify(&mapped_call, signature) => {
					// We only want to mark unsigned submit_processing_result_hash extrinsics as
					// valid if the signature sent in the body is verified by the given public
					// key
					ValidTransaction::with_tag_prefix("LogicProviderCommit")
						// We set base priority to 2**20 and hope it's included before any
						// other transactions in the pool. Next we tweak the priority
						// depending on how much it differs from the current average. (the
						// more it differs the more priority it has).
						.priority(TransactionPriority::MAX)
						// The transaction is only valid for next 5 blocks. After that it's
						// going to be revalidated by the pool.
						.longevity(T::RevealWindowLength::get() as u64)
						// It's fine to propagate that transaction to other peers, which
						// means it can be created even by nodes that don't produce blocks.
						// Note that sometimes it's better to keep it for yourself (if you
						// are the block producer), since for instance in some schemes
						// others may copy your solution and claim a reward.
						.propagate(true)
						.and_provides((public.clone(), *commit_hash))
						.build()
				},
				// In case of unverified signature, or bad decoding of the account:
				_ => InvalidTransaction::BadSigner.into(),
			}
		}

		// Specific checks for the unique calls expected by the logic provider pallet
		fn verify_call_public_key_reveal(
			public: &Public,
			random_seed: &u8,
			reveal_hash: &<T as pallet_commitments::Config>::Hash,
			mapped_call: &[u8],
			signature: &Signature,
		) -> TransactionValidity {
			match Pallet::<T>::to_account_id(public.clone()) {
				Ok(_account) if public.verify(&mapped_call, signature) => {
					ValidTransaction::with_tag_prefix("LogicProviderReveal")
						// We set base priority to 2**20 and hope it's included before any
						// other transactions in the pool. Next we tweak the priority
						// depending on how much it differs from the current average. (the
						// more it differs the more priority it has).
						.priority(TransactionPriority::MAX)
						// The transaction is only valid for next 5 blocks. After that it's
						// going to be revalidated by the pool.
						.longevity(T::RevealWindowLength::get() as u64)
						// It's fine to propagate that transaction to other peers, which
						// means it can be created even by nodes that don't produce blocks.
						// Note that sometimes it's better to keep it for yourself (if you
						// are the block producer), since for instance in some schemes
						// others may copy your solution and claim a reward.
						.propagate(true)
						.and_provides((public.clone(), *random_seed, *reveal_hash))
						.build()
				},
				_ => InvalidTransaction::BadSigner.into(),
			}
		}
	}

	impl<T> Pallet<T>
	where
		// We use `offchain::SendTransactionTypes` for unsigned extrinsic creation and
		// submission.
		T: Config + frame_system::offchain::SendTransactionTypes<Call<T>>,
	{
		#[allow(clippy::result_unit_err)]
		pub fn create_extrinsic_from_external_call(
			payload: Vec<u8>,
			public: Public,
			signature: Signature,
		) -> Result<(), ()>
		where
			<T as pallet_commitments::Config>::Hash: From<sp_core::H256>,
		{
			use frame_system::offchain::SubmitTransaction;
			let external_call = MapToCall::decode(&mut &payload[..]).unwrap();
			let call = match external_call {
				MapToCall::LogicProviderCall(LogicProviderCall::CommitHash { .. }) =>
					Call::commit_processing_result_hash { payload, signature, public },
				MapToCall::LogicProviderCall(LogicProviderCall::RevealHash { .. }) =>
					Call::reveal_processing_result_hash { payload, signature, public },
			};

			let result =
				SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.clone().into());

			match result {
				Ok(()) => log::info!(
					target: "runtime::logic-provider",
					"Submitted hash {:?}.",
					call
				),
				Err(e) => log::error!(
					target: "runtime::logic-provider",
					"Error submitting hash ({:?}): {:?}",
					call,
					e,
				),
			}
			result
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		/// Validate unsigned call to this module.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::commit_processing_result_hash { public, payload, signature } => {
					let decoded_call = MapToCall::decode(&mut &payload[..]).unwrap();

					if let MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
						hash,
						..
					}) = decoded_call
					{
						Pallet::<T>::verify_call_public_key_commit(
							public,
							&hash.into(),
							payload,
							signature,
						)
					} else {
						InvalidTransaction::Call.into()
					}
				},

				Call::reveal_processing_result_hash { public, payload, signature } => {
					let decoded_call = MapToCall::decode(&mut &payload[..]).unwrap();

					if let MapToCall::LogicProviderCall(LogicProviderCall::RevealHash {
						reveal_hash,
						random_seed,
						..
					}) = decoded_call
					{
						Pallet::<T>::verify_call_public_key_reveal(
							public,
							&random_seed,
							&reveal_hash.into(),
							payload,
							signature,
						)
					} else {
						InvalidTransaction::Call.into()
					}
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}
