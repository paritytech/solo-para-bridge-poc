pub use sp_std::vec::Vec;

pub enum SharedStateError {
	InputTooLong,
}

/// The trait that allows us to read some state from some storage
/// and write some state to it.
/// Should always use the `I` generic to refer to the correct storage.
pub trait SharedState<I: 'static> {
	fn read() -> Vec<u8>;
	fn write(state: Vec<u8>) -> Result<(), SharedStateError>;
}

// Blanket implementation for x-chain pallets on solo chain side.
impl<I: 'static> SharedState<I> for () {
	fn read() -> Vec<u8> {
		Vec::new()
	}

	fn write(_state: Vec<u8>) -> Result<(), SharedStateError> {
		Ok(())
	}
}
