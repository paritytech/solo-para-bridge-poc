
build-all:
	cargo b -r -p rialto-parachain-collator && \
	cargo b -r -p millau-bridge-node && \
	cargo b -r -p rialto-bridge-node && \
	cargo build -r -p substrate-relay

start-para-mac:
	./deployments/zombienet/zombienet-macos -p native spawn ./deployments/zombienet/rialto-parachain.toml

start-para-linux:
	./deployments/zombienet/zombienet-linux -p native spawn ./deployments/zombienet/rialto-parachain.toml

start-millau:
	./deployments/local-scripts/run-millau-node.sh

start-header-relay:
	MILLAU_PORT=9944 \
	RIALTO_PORT=$(rialto-port)  \
	RIALTO_PARACHAIN_PORT=$(rialto-parachain-port)  \
	./deployments/local-scripts/relay-headers-and-messages-millau-rialto-parachain.sh
