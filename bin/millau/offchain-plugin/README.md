# Example Logic Provider Module

### Summary
The goal of this project is to demonstrate an example integration of the imported module that is external to the Logic Provider client:
1. Start a number of long-running services with custom logic
2. Expose the required public functions to the Substrate client. The client will run those tasks in parallel.

Inside the module, we show some example business logic. For the sake of example, we hash some local file.
The purpose of the hash is to represent the input of the Logic Provider pallet, which accepts some hash result.

### Start Runtime Interface Service
The service should execute some long-running task, which retrieves offchain data. The example module shows one way to do this with a tokio interval:
```rust
	let interval = tokio::time::interval(Duration::from_secs(27));
	tokio_stream::wrappers::IntervalStream::new(interval)
		.for_each(|now| {
            // for each interval "tick", perform some business logic
		}).await
```

### Expose Start Function
It is required to expose a public function called `start`. This function is expected to execute some business logic to retrieve offchain
data based on some interval.
It accepts the `Client` instance, which we'll use to do calls to the runtime, as well as `Keystore` and `LocalStorage`.
They will be used to get the config & write intermediate data to offchain storage.

Generally, we group the runtime apis by traits: `ConstructExtrinsicApi`, `StorageQueryApi` and so on.
We will put such trait bounds on `start` for each runtime api we're going to call.
So, whenever you're going to add any runtime APIs, you'll also need to update the `where` bounds
on `Client` in the `offchain-plugin`.

```toml
#In your Cargo.toml, import the necessary local crates
runtime-api = { version = "0.2.0", path = "../primitives/runtime-api" }
primitives = { version = "0.2.0", path = "../primitives/primitives" }
```

```rust
use primitives::shared::MapToCall;
// ...
pub async fn start<B, C: 'static>(
	// Accept some closure that expects a `MapToCall`
	client: Arc<C>,
	offchain_storage: Arc<Mutex<LocalStorage>>,
	keystore: Arc<LocalKeystore>,
) where
		B: BlockT,
		C: ProvideRuntimeApi<B> + HeaderBackend<B>,
		C::Api: ConstructExtrinsicApi<B>, // this trait is used to connect `submit_unchecked_extrinsic` runtime API to the client
{
	// Indicate some arbitrary seconds interval, where for each "tick" the business logic will be
	// invoked
	// Fetch a JSON object with various values configured by the node operator. In addition, it contains the first
	// valid public key set by the node operator, set in the key `config_account_id`.
	let config: Arc<Mutex<Map<String, Value>>> = get_config(&offchain_storage, &keystore).await;
	let interval = interval(Duration::from_secs(27));
	let start = Instant::now();
	let pair = get_keypair(&config, &keystore)
			.await
			.expect("Could not get pair from the keystore");
	run_service::<B, C>(client, pair, config, start, interval, &offchain_storage).await
}
```
The `B` type parameter stands for the `Block` type, and the `C` type parameter - for our `Client` type.

### Define additional functions to be used in the runtime
In our case, the `start` function is not magic. You can name it how you want, but in this implementation,
it serves as the entrypoint for out logic provider flow.

Apart from the `start` function, we use `poll_reveal_window_state` function in [state_poller.rs](./src/state_poller.rs):
```rust
pub async fn poll_reveal_window_state<B, C>(
	_offchain_storage: Arc<Mutex<LocalStorage>>,
	_client: Arc<C>,
	_keystore: Arc<LocalKeystore>,
) where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	C::Api: StorageQueryApi<B> + ConstructExtrinsicApi<B>,
{
	// code goes here
}
```

We then launch it as a substrate task in node's [`serivce.rs`](../node/src/service.rs).
To summarize - you can define as many public functions as you need. Just don't forget to use
them in [`serivce.rs`](../node/src/service.rs)!

### Creating/Sending Transactions to the Substrate Runtime
The `start` function exposed by the module accepts the substrate client.
Call the runtime apis to send calls to the runtime. To do this, first create a call to be dispatched in the runtime:
```rust
	// Construct a call to be dispatched into the runtime
let call =
	MapToCall::LogicProviderCall(LogicProviderCall::CommitHash { metadata_id, hash });
```
And then execute this call using the function in [calls.rs](./src/calls.rs):
```rust
	// Send the call to the runtime.
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
				&generic::BlockId::Hash(best_hash),
				payload,
				signature,
				pair.public(),
			)
			.unwrap()
}
// in service.rs:
submit_call(client, pair, call);

```
