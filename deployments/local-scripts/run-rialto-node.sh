#!/bin/bash

# Run a development instance of the Rialto Substrate bridge node.
# To override the default port just export RIALTO_PORT=9944

RIALTO_PORT="${RIALTO_PORT:-9945}"

RUST_LOG=runtime=trace \
    ./target/release/rialto-bridge-node --dev --tmp \
    --rpc-cors=all --rpc-port 9934 --unsafe-rpc-external --unsafe-ws-external \
    --port 33034 --ws-port $RIALTO_PORT
