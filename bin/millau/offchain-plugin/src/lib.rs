mod calls;
pub mod config;
mod logic_provider;
mod offchain;
mod service;
mod state_poller;
pub use service::start;
pub use state_poller::poll_reveal_window_state;

// A composite error enum for our offchain plugin.
#[derive(Debug)]
pub enum PluginError {
	/// The key for the node was not found in the keystore.
	KeyNotFound,
	/// Something went wrong when accessing the keystore.
	KeystoreError,
	/// Couldn't convert the `AccountId`s when invoking the runtime api.
	AccountConversionError,
	/// Generic runtime api error.
	RuntimeApiError,
	/// Something went wrong while encoding/decoding.
	CodecError(codec::Error),
}
