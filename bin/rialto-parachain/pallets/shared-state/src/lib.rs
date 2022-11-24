#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use bin_traits::{SharedState, SharedStateError, Vec};
	use codec::{Decode, Encode};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		#[pallet::constant]
		type MaxStateLength: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	impl<T: Config<I>, I: 'static> SharedState<I> for Pallet<T, I> {
		fn read() -> Vec<u8> {
			State::<T, I>::get().into_inner()
		}

		fn write(state: Vec<u8>) -> Result<(), SharedStateError> {
			State::<T, I>::put(
				BoundedVec::try_from(state).map_err(|_| SharedStateError::InputTooLong)?,
			);
			Ok(())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn get_state)]
	pub type State<T: Config<I>, I: 'static = ()> =
		StorageValue<_, BoundedVec<u8, T::MaxStateLength>, ValueQuery>;
}
