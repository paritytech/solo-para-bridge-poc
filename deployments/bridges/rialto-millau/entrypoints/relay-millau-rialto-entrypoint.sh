#!/bin/bash
set -xeu

sleep 15

# see `rialto_millau_bridge_identifier_did_not_changed` test in `millau-runtime` crate for
# details on how this lane is computed
MESSAGE_LANE="52011894c856c0c613a2ad2395dfbb509090f6b7a6aef9359adb75aa26a586c7"

/home/user/substrate-relay init-bridge millau-to-rialto \
	--source-host millau-node-alice \
	--source-port 9944 \
	--target-host rialto-node-alice \
	--target-port 9944 \
	--target-signer //Sudo

/home/user/substrate-relay init-bridge rialto-to-millau \
	--source-host rialto-node-alice \
	--source-port 9944 \
	--target-host millau-node-alice \
	--target-port 9944 \
	--target-signer //Sudo

# Give chain a little bit of time to process initialization transaction
sleep 6

RIALTO_NODE_CONNECTION_PARAMS=$([ -z ${GLOBAL_DEPLOYMENTS} ] && \
	echo "--rialto-host rialto-node-alice --rialto-port 9944" \
	|| \
	echo "--rialto-host wss.rialto.brucke.link --rialto-port 443 --rialto-secure" )

/home/user/substrate-relay relay-headers-and-messages millau-rialto \
	--millau-host millau-node-alice \
	--millau-port 9944 \
	--millau-signer //Rialto.HeadersAndMessagesRelay \
	--millau-transactions-mortality=64 \
	$RIALTO_NODE_CONNECTION_PARAMS \
	--rialto-signer //Millau.HeadersAndMessagesRelay \
	--rialto-transactions-mortality=64 \
	--lane=$MESSAGE_LANE \
	--prometheus-host=0.0.0.0
