use codec::Encode;
use primitives::shared::{MapToCall, Pair};
use runtime_api::ConstructExtrinsicApi;
use sc_client_api::HeaderBackend;
use sp_api::{BlockT, ProvideRuntimeApi};
use sp_core::Pair as _;
use sp_runtime::generic;
use std::sync::Arc;

/// Submit a call to the runtime.
///
/// This function is used to send a variant of `MapToCall`
/// to the runtime. Upon submission, it will be processed by the
/// respective runtime api impl in `runtime` and dispatched to
/// the respective pallet.
pub fn submit_call<B, C: 'static>(
	client: Arc<C>,
	pair: Arc<Pair>,
	mapped_call: MapToCall,
) -> Result<(), ()>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B>,
	C::Api: ConstructExtrinsicApi<B>,
{
	let best_hash = client.info().best_hash;
	let payload = mapped_call.encode();
	let signature = pair.sign(&payload);
	client
		.runtime_api()
		// Submit our call to the runtime api
		.submit_unchecked_extrinsic(
			best_hash,
			payload,
			signature,
			pair.public(),
		)
		.map_err(|_| ())?
}
