#!/bin/bash

# A script for relaying Millau headers to the Rialto chain.
#
# Will not work unless both the Rialto and Millau are running (see `run-rialto-node.sh`
# and `run-millau-node.sh).

MILLAU_PORT="${MILLAU_PORT:-9945}"
RIALTO_PORT="${RIALTO_PORT:-9944}"

RUST_LOG=bridge=debug \
./target/release/substrate-relay init-bridge millau-to-rialto-parachain \
	--source-host localhost \
	--source-port $MILLAU_PORT \
	--target-host localhost \
	--target-port $RIALTO_PORT \
	--target-signer //Sudo \
	--source-version-mode Bundle \
	--target-version-mode Bundle

sleep 5
RUST_LOG=bridge=debug \
./target/release/substrate-relay relay-headers millau-to-rialto-parachain \
	--source-host localhost \
	--source-port $MILLAU_PORT \
	--target-host localhost \
	--target-port $RIALTO_PORT \
	--target-signer //Sudo \
	--prometheus-host=0.0.0.0
