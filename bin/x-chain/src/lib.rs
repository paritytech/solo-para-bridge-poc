#![cfg_attr(not(feature = "std"), no_std)]

pub mod types;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::State;
	use bin_traits::{SharedState, SharedStateError};
	use frame_support::{inherent::Vec, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use xcm::latest::prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type RuntimeEvent: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type SharedStateAccess: SharedState<I>;
		/// Origins we allow to respond with a query.
		type CrossChainOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = MultiLocation,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		ExecutedTransactWith(Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		InputTooLong,
	}

	impl<T, I> From<SharedStateError> for Error<T, I> {
		fn from(error: SharedStateError) -> Self {
			match error {
				SharedStateError::InputTooLong => Self::InputTooLong,
			}
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		// Todo: benchmarking for this 🤔
		#[pallet::weight(0)]
		pub fn placeholder(origin: OriginFor<T>, proof: Vec<u8>) -> DispatchResult {
			let _loc = T::CrossChainOrigin::ensure_origin(origin)?;
			T::SharedStateAccess::write(proof.clone()).map_err(|e| -> Error<T, I> { e.into() })?;
			Self::deposit_event(Event::<T, I>::ExecutedTransactWith(proof));
			Ok(())
		}
	}
}
