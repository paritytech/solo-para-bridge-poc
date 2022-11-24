#!/bin/bash

# Used for manually sending a message to a running network.
#
# You could for example spin up a full network using the Docker Compose files
# we have (to make sure the message relays are running), but remove the message
# generator service. From there you may submit messages manually using this script.

MILLAU_PORT="${MILLAU_PORT:-9944}"

RUST_LOG=runtime=trace,substrate-relay=trace,bridge=trace \
  ./target/release/substrate-relay send-message millau-to-rialto-parachain \
  	--source-host localhost \
		--source-port $MILLAU_PORT \
		--source-signer //Alice \
		--lane 00000000 \
		raw 020419ac