# Reusing the Testnet node for different logic

This node contains several parts that must be changed to extend/alter
the business logic functionality.

So, how do we update the code to completely change the logic?

## Proposed use case

Let's say we need to create some oracle network. The nodes will fetch the
BTC/USDC rate from some off-chain source, send it (as `u128`) on-chain
and calculate the average. Within this use case we do a completely different
thing (as opposed to the envelope processing), so let's go over the changes
we need to do to change the blockchain.

### Updating the `primitives`

First, we'll need to update the `bin/millau/primitives` directory, mainly the
`primitives` crate present inside this directory.

Let's say [this](../bin/millau/primitives/primitives/src/lib.rs) is the code we're looking at right now.
```rust
#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
pub enum LogicProviderCall { // the individual pallet enum
	CommitHash { metadata_id: MetadataId, hash: Hash },
        RevealHash { reveal_hash: Hash, random_seed: u8, metadata_id: MetadataId, },
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
pub enum MapToCall { // aggregated pallets & calls type
	LogicProviderCall(LogicProviderCall),
}
```

Let's rework it to match our price processing logic:

```rust
// The pallet is now called pallet_price_processor
#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
pub enum PriceProcessorCall {
	SubmitPrice { price: u128 },
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, TypeInfo)]
pub enum MapToCall {
    PriceProcessorCall(PriceProcessorCall),
}
```
Additionally, we can change the defined type aliases and whatnot accordingly.
Moreover, the key types can also be changed, but it's undesirable to do.
Now we're done updating the primitives.

### Updating the `offchain-plugin`

The client module is the place where we have all offchain logic defined.

Let's look at an example implementation of the client module:
(documentation & auth stuff omitted)
```rust
async fn run_service<B, C: 'static>(
    client: Arc<C>,
    pair: Arc<Pair>,
    config: Arc<Mutex<Map<String, Value>>>,
    start: Instant,
    interval: Interval,
    offchain_storage: &Arc<Mutex<LocalStorage>>,
) where
    B: BlockT,
    C: ProvideRuntimeApi<B> + HeaderBackend<B>,
    C::Api: ConstructExtrinsicApi<B>,
{
    tokio_stream::wrappers::IntervalStream::new(interval)
        .for_each(|now| {
            let client = client.clone();
            let pair = pair.clone();
            let config = config.clone();
            let elapsed = now.duration_since(start).as_secs_f32();
            let metadata_id = elapsed.trunc() as u64;
            async move {
                let config = config.lock().await;
                if let Some(reveal_hash) = logic_provider::get_data(config) {
                    let (commit_hash, random_seed) =
                        logic_provider::create_commit_hash(reveal_hash);

                    let call = MapToCall::LogicProviderCall(LogicProviderCall::CommitHash {
                        metadata_id,
                        hash: commit_hash,
                    });

                    submit_call(client, pair, call);
                }
            }
        })
        .await
}
```

The part we want to change is the lines after `async move`.
In the previous use case, we get some data using the config.
This config will provide us with the data source, which we'll use to fetch
the price.
Generally, we should get the computation result & feed it to the call.
The following is just a usage example:
```rust
// some logic
async move {
    // price processor is the offchain module in this case
    if let Some(price) = price_processor::get_price(config) {
        let call = MapToCall::PriceProcessorCall(PriceProcessorCall::SubmitPrice {
            price,
        });
        submit_call(client, pair, call);
    }
}
// some logic
```

Everything else may be left unchanged, and the unused variables may be
cleaned up.

We're finished with the offchain plugin updates.

### Updating the `pallets` and `runtime`

This is the part where we'll probably write most code.
We will need to create our pallet which matches our logic.

Although the actual logic is irrelevant for us for now, let's suggest we
have some dispatchable with signature
`submit_price(origin: OriginFor<T>, price: u128)`.
Now, as we create this call from the runtime API, we need to cross this
boundary between the "business logic" call & the actual dispatchable.
We will do this using the following function (in [runtime definition file](../bin/millau/runtime/src/lib.rs)):
```rust
impl construct_extrinsic::ConstructExtrinsicApi<Block> for Runtime {
		fn submit_unchecked_extrinsic(
			mapped_call: Vec<u8>,
			signature: primitives::Signature,
			public: primitives::Public,
		) -> Result<(), ()> {
			if public.verify(&mapped_call, &signature) {
				let decoded_call = MapToCall::decode(&mut &mapped_call[..]).map_err(|_| ())?;
				match decoded_call {
                    // process other potential variants
					MapToCall::PriceProcessorCall(price_processor_call) =>
						PriceProcessor::create_extrinsic_from_external_call(price_processor_call, public),
                    // call their respective dispatch functions
				}
			} else {
				Err(())
			}
		}
	}
```

The `create_extrinsic_from_external_call` functions acts like a call dispatch function:
we construct the `pallet::Call` enum using the data that we got via the runtime API, and
dispatch the call with the `frame_system::offchain::SubmitTransaction` trait.

The `create_extrinsic_from_external_call` function implementation (in [pallet](../bin/millau/pallets/logic-provider/src/lib.rs)):

```rust
impl<T> Pallet<T>
	where
		T: Config + frame_system::offchain::SendTransactionTypes<Call<T>>,
	{
		pub fn create_extrinsic_from_external_call(
			price_processor_call: primitives::PriceProcessorCall,
			_public: primitives::Public, // we will not use the pubkey as per our example
		) -> Result<(), ()>
		where
			<T as Config>::Hash: From<sp_core::H256>,
		{
			use frame_system::offchain::SubmitTransaction;
			let call = match price_processor_call {
				primitives::PriceProcessorCall::SubmitPrice { price } =>
					Call::submit_price { price },
			};
			match SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.clone().into())
			{
				Ok(()) => log::info!(
					target: "runtime::price-processor",
					"Submitted hash {:?}.",
					call
				),
				Err(e) => log::error!(
					target: "runtime::price-processor",
					"Error submitting hash ({:?}): {:?} \n Was the account submitted verified in the registry pallet?",
					call,
					e,
				),
			}

			Ok(())
		}
	}
```

We're done with updating the runtime API-related stuff, but to make the
flow complete, we'll also need to do the following:

1. Implement `pallet_price_processor::Config` for Runtime;
2. Add the pallet to `construct_runtime!` macro invocation.

### Adding some other pallet

Adding some other pallet is basically the same flow as it is with changing
the blockchain logic & the "logic provider" module.
Instead of removing/renaming old code, one should:
1. Add the enum of desired calls, add the pallet type to the `MapToCall` enum;
2. Update the client module to execute that call;
3. Add the pallet, add `construct_extrinsic_from_external_call` function;
4. Match on the pallet type in the runtime/lib.rs file, forward the call
to the `construct_extrinsic_from_external_call` implementation in the
respective pallet.

Voilà! Our stuff is easy to reuse, enjoy!
