#!/bin/bash

# Run a development instance of the Millau Substrate bridge node.
# To override the default port just export MILLAU_PORT=9945

MILLAU_PORT="${MILLAU_PORT:-9944}"

RUST_LOG=runtime=trace \
./target/release/millau-bridge-node --dev --tmp \
    --node-processing-role logic-provider --set-config bin/millau/offchain-plugin/localConfig.json \
    --rpc-cors=all  --unsafe-ws-external \
    --port 33043 --rpc-port 9934 --ws-port $MILLAU_PORT

# --unsafe-rpc-external
