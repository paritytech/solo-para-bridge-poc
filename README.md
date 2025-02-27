# Solo-Para Bridge PoC

## Overview
This repository is based on the `Parity Bridges Common` [repository](https://github.com/paritytech/parity-bridges-common) for a spike/POC on bridging between a solo chain and a parachain.

Here is the list of major changes on top of Parity Bridges Common:

- `./bin/millau` is the solo chain and contains some additional on-chain and off-chain functionality:
  - Off-chain is responsible for retrieving some off-chain data and submitting it to the on-chain for further processing.
  - After getting off-chain data, on-chain will do computation based on the business logic, and then send the final result to parachain.
  - For more info please have a look at the millau's [readme](bin/millau/README.md).
- `./bin/rialto-parachain` is the parachain and contains a pallet which is responsible for storing the data received from the solo-chain.
- Add XCM bridging example to the runtimes/pallets.

### Note:
This POC is a forked version of [parity-common-bridges](https://github.com/paritytech/parity-bridges-common) and highly dependent on it.

The versions [bin](bin) module follows currently are:
- `Substrate: polkadot-v0.9.32`
- `Cumulus: polkadot-v0.9.32`
- `Polkadot: release-v0.9.38`

This POC's version upgrades are highly based on [parity-common-bridges](https://github.com/paritytech/parity-bridges-common).
We'll keep the POC up-to-date with the latest polkadot versions once its parent repository upgrades itself.

## Some notes on running

Take a look through the docs.

## Deploying `rialto-parachain` with zombienet

Before doing that, run this (builds binaries that are relevant for us):
```bash
make build-all
```

### Note:  Due to maintenance, zombienet is not functional at the moment for this repo. For spinning up the `rialto-parachain` network we need to use a traditional approach which is defined [here](https://docs.substrate.io/tutorials/build-a-parachain/).
1. Build `rialto-bridge-node` in release mode
2. Download `zombienet` binary release from `zombienet` [GitHub](https://github.com/paritytech/zombienet) page
3. Move this binary to [this directory](deployments/zombienet)
4. Configure the `rialto-bridge-node` relay chain binary path [here](deployments/zombienet/rialto-parachain.toml)
5. Start with `make start-para(mac|linux)`
6. Copy the port listed for collator1, which you will use for the `rialto-parachain-port` in a later command. Copy Alice's, Bob's, or Charlie's port, as you will use it for the `rialto-port` in a later command.

## Starting the relay process
1. Run millau node (`make start-millau`)
2. Wait until the parachain is onboarded and starts producing blocks
3. Run `make start-header-relay rialto-port={your port here} rialto-parachain-port={your other port here}`,

## Observe the Pallet's message sending
The existing LP pallet in the Millau runtime utilizes the Millau + Rialto-Parachain XCM configuration to send bridged XCM messages. It tries to do this after it reaches a certain internal state(See [README](bin/millau/README.md) for more). To quickly start the message-sending process:
1. Open the Millau node in Polkadotjs
2. Got to Accounts, copy Alice's public key
3. Go to Developer > RPC Calls > author > insertKey
4. keyType: pubK, suri: //Alice, publicKey (paste the key from your clipboard)
5. Submit and observe the events. Wait for several rounds (HashRevealed events) to pass.
6. In the Rialto parachain's polkadotjs, observe the xcm messages noted in the events

## The docs

This is a collection of components for building bridges.

These components include Substrate pallets for syncing headers, passing arbitrary messages, as well
as libraries for building relayers to provide cross-chain communication capabilities.

Three bridge nodes are also available. The nodes can be used to run test networks which bridge other
Substrate chains.

🚧 The bridges are currently under construction - a hardhat is recommended beyond this point 🚧

## Contents

- [Installation](#installation)
- [High-Level Architecture](#high-level-architecture)
- [Project Layout](#project-layout)
- [Running the Bridge](#running-the-bridge)
- [How to send a message](#how-to-send-a-message)
- [Community](#community)

## Installation

To get up and running you need both stable and nightly Rust. Rust nightly is used to build the Web
Assembly (WASM) runtime for the node. You can configure the WASM support as so:

```bash
rustup install nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

Once this is configured you can build and test the repo as follows:

```
git clone https://github.com/paritytech/parity-bridges-common.git
cd parity-bridges-common
cargo build --all
cargo test --all
```

Also you can build the repo with
[Parity CI Docker image](https://github.com/paritytech/scripts/tree/master/dockerfiles/bridges-ci):

```bash
docker pull paritytech/bridges-ci:production
mkdir ~/cache
chown 1000:1000 ~/cache #processes in the container runs as "nonroot" user with UID 1000
docker run --rm -it -w /shellhere/parity-bridges-common \
                    -v /home/$(whoami)/cache/:/cache/    \
                    -v "$(pwd)":/shellhere/parity-bridges-common \
                    -e CARGO_HOME=/cache/cargo/ \
                    -e SCCACHE_DIR=/cache/sccache/ \
                    -e CARGO_TARGET_DIR=/cache/target/  paritytech/bridges-ci:production cargo build --all
#artifacts can be found in ~/cache/target
```

If you want to reproduce other steps of CI process you can use the following
[guide](https://github.com/paritytech/scripts#reproduce-ci-locally).

If you need more information about setting up your development environment [Substrate's
Installation page](https://docs.substrate.io/main-docs/install/) is a good
resource.

## High-Level Architecture

This repo has support for bridging foreign chains together using a combination of Substrate pallets
and external processes called relayers. A bridge chain is one that is able to follow the consensus
of a foreign chain independently. For example, consider the case below where we want to bridge two
Substrate based chains.

```
+---------------+                 +---------------+
|               |                 |               |
|     Rialto    |                 |    Millau     |
|               |                 |               |
+-------+-------+                 +-------+-------+
        ^                                 ^
        |       +---------------+         |
        |       |               |         |
        +-----> | Bridge Relay  | <-------+
                |               |
                +---------------+
```

The Millau chain must be able to accept Rialto headers and verify their integrity. It does this by
using a runtime module designed to track GRANDPA finality. Since two blockchains can't interact
directly they need an external service, called a relayer, to communicate. The relayer will subscribe
to new Rialto headers via RPC and submit them to the Millau chain for verification.

Take a look at [Bridge High Level Documentation](./docs/high-level-overview.md) for more in-depth
description of the bridge interaction.

## Project Layout

Here's an overview of how the project is laid out. The main bits are the `bin`, which is the actual
"blockchain", the `modules` which are used to build the blockchain's logic (a.k.a the runtime) and
the `relays` which are used to pass messages between chains.

```
├── bin             // Node and Runtime for the various Substrate chains
│  └── ...
├── deployments     // Useful tools for deploying test networks
│  └──  ...
├── modules         // Substrate Runtime Modules (a.k.a Pallets)
│  ├── beefy        // On-Chain BEEFY Light Client (in progress)
│  ├── grandpa      // On-Chain GRANDPA Light Client
│  ├── messages     // Cross Chain Message Passing
│  ├── parachains   // On-Chain Parachains Light Client
│  ├── relayers     // Relayer rewards registry
│  └──  ...
├── primitives      // Code shared between modules, runtimes, and relays
│  └──  ...
├── relays          // Application for sending finality proofs and messages between chains
│  └──  ...
└── scripts         // Useful development and maintenance scripts
```

## Running the Bridge

To run the Bridge you need to be able to connect the bridge relay node to the RPC interface of nodes
on each side of the bridge (source and target chain).

There are 2 ways to run the bridge, described below:

- building & running from source: with this option, you'll be able to run the bridge between two standalone
chains that are running GRANDPA finality gadget to achieve finality;

- running a Docker Compose setup: this is a recommended option, where you'll see bridges with parachains,
complex relays and more.

### Using the Source

First you'll need to build the bridge nodes and relay. This can be done as follows:

```bash
# In `parity-bridges-common` folder
cargo build -p rialto-bridge-node
cargo build -p millau-bridge-node
cargo build -p substrate-relay
```

### Running a Dev network

We will launch a dev network to demonstrate how to relay a message between two Substrate based
chains (named Rialto and Millau).

To do this we will need two nodes, two relayers which will relay headers, and two relayers which
will relay messages.

#### Running from local scripts

To run a simple dev network you can use the scripts located in the
[`deployments/local-scripts` folder](./deployments/local-scripts).

First, we must run the two Substrate nodes.

```bash
# In `parity-bridges-common` folder
./deployments/local-scripts/run-rialto-node.sh
./deployments/local-scripts/run-millau-node.sh
```

After the nodes are up we can run the header relayers.

```bash
./deployments/local-scripts/relay-millau-to-rialto.sh
./deployments/local-scripts/relay-rialto-to-millau.sh
```

At this point you should see the relayer submitting headers from the Millau Substrate chain to the
Rialto Substrate chain.

```
# Header Relayer Logs
[Millau_to_Rialto_Sync] [date] DEBUG bridge Going to submit finality proof of Millau header #147 to Rialto
[...] [date] INFO bridge Synced 147 of 147 headers
[...] [date] DEBUG bridge Going to submit finality proof of Millau header #148 to Rialto
[...] [date] INFO bridge Synced 148 of 149 headers
```

Finally, we can run the message relayers.

```bash
./deployments/local-scripts/relay-messages-millau-to-rialto.sh
./deployments/local-scripts/relay-messages-rialto-to-millau.sh
```

You will also see the message lane relayers listening for new messages.

```
# Message Relayer Logs
[Millau_to_Rialto_MessageLane_0x52011894c856c0c613a2ad2395dfbb509090f6b7a6aef9359adb75aa26a586c7] [date] DEBUG bridge Asking Millau::ReceivingConfirmationsDelivery about best message nonces
[...] [date] INFO bridge Synced Some(2) of Some(3) nonces in Millau::MessagesDelivery -> Rialto::MessagesDelivery race
[...] [date] DEBUG bridge Asking Millau::MessagesDelivery about message nonces
[...] [date] DEBUG bridge Received best nonces from Millau::ReceivingConfirmationsDelivery: TargetClientNonces { latest_nonce: 0, nonces_data: () }
[...] [date] DEBUG bridge Asking Millau::ReceivingConfirmationsDelivery about finalized message nonces
[...] [date] DEBUG bridge Received finalized nonces from Millau::ReceivingConfirmationsDelivery: TargetClientNonces { latest_nonce: 0, nonces_data: () }
[...] [date] DEBUG bridge Received nonces from Millau::MessagesDelivery: SourceClientNonces { new_nonces: {}, confirmed_nonce: Some(0) }
[...] [date] DEBUG bridge Asking Millau node about its state
[...] [date] DEBUG bridge Received state from Millau node: ClientState { best_self: HeaderId(1593, 0xacac***), best_finalized_self: HeaderId(1590, 0x0be81d...), best_finalized_peer_at_best_self: HeaderId(0, 0xdcdd89...) }
```

To send a message see the ["How to send a message" section](#how-to-send-a-message).

### How to send a message

In this section we'll show you how to quickly send a bridge message. The message is just an encoded XCM
`Trap(43)` message.

```bash
# In `parity-bridges-common` folder
./scripts/send-message-from-millau-rialto.sh
```

After sending a message you will see the following logs showing a message was successfully sent:

```
INFO bridge Sending message to Rialto. Size: 11.
TRACE bridge Sent transaction to Millau node: 0x5e68...
```

And at the Rialto node logs you'll something like this:

```
... runtime::bridge-messages: Received messages: total=1, valid=1. Weight used: Weight(ref_time: 1215065371, proof_size: 48559)/Weight(ref_time: 1215065371, proof_size: 54703).
```

It means that the message has been delivered and dispatched. Message may be dispatched with an
error, though - the goal of our test bridge is to ensure that messages are successfully delivered
and all involved components are working.

## Full Network Docker Compose Setup

For a more sophisticated deployment which includes bidirectional header sync, message passing,
monitoring dashboards, etc. see the [Deployments README](./deployments/README.md).

You should note that you can find images for all the bridge components published on
[Docker Hub](https://hub.docker.com/u/paritytech).

To run a Rialto node for example, you can use the following command:

```bash
docker run -p 30333:30333 -p 9933:9933 -p 9944:9944 \
  -it paritytech/rialto-bridge-node --dev --tmp \
  --rpc-cors=all --unsafe-rpc-external
```

## Community

Main hangout for the community is [Element](https://element.io/) (formerly Riot). Element is a chat
server like, for example, Discord. Most discussions around Polkadot and Substrate happen
in various Element "rooms" (channels). So, joining Element might be a good idea, anyway.

If you are interested in information exchange and development of Polkadot related bridges please
feel free to join the [Polkadot Bridges](https://app.element.io/#/room/#bridges:web3.foundation)
Element channel.

The [Substrate Technical](https://app.element.io/#/room/#substrate-technical:matrix.org) Element
channel is most suited for discussions regarding Substrate itself.
