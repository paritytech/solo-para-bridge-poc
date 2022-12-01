../../../target/release/temlpate-node benchmark pallet --chain dev --execution wasm \
    --wasm-execution compiled \
    --pallet pallet_logic_provider \
    --extrinsic '*' \
    --steps 25 \
    --repeat 25 \
    --json-file=benchmark_raw.json \
    --output ./weights_new.rs
