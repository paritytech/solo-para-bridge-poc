#!/bin/bash
set -xeu

MILLAU_PORT="${MILLAU_PORT:-9944}"
RIALTO_PORT="${RIALTO_PORT:-9945}"
RIALTO_PARACHAIN_PORT="${RIALTO_PARACHAIN_PORT:-9946}"
RUST_LOG=bridge=debug ./target/release/substrate-relay init-bridge millau-to-rialto-parachain \
	--source-host localhost \
	--source-port $MILLAU_PORT \
	--target-host localhost \
	--target-port $RIALTO_PARACHAIN_PORT \
	--target-signer //Sudo
sleep  5
RUST_LOG=bridge=debug ./target/release/substrate-relay init-bridge rialto-to-millau \
	--source-host localhost \
	--source-port $RIALTO_PORT \
	--target-host localhost \
	--target-port $MILLAU_PORT \
	--target-signer //Sudo

# Give chain a little bit of time to process initialization transaction
sleep 6

RUST_LOG=bridge=debug ./target/release/substrate-relay relay-headers-and-messages millau-rialto-parachain \
	--millau-host localhost \
	--millau-port $MILLAU_PORT \
	--millau-signer //Alice \
	--rialto-headers-to-millau-signer //Alice \
	--millau-messages-pallet-owner=//Sudo \
	--rialto-parachain-host localhost \
	--rialto-parachain-port $RIALTO_PARACHAIN_PORT \
	--rialto-parachain-signer //Alice \
	--rialto-parachain-messages-pallet-owner=//Sudo \
	--rialto-host localhost \
	--rialto-port $RIALTO_PORT \
	--lane=00000000 \
	--prometheus-host=0.0.0.0