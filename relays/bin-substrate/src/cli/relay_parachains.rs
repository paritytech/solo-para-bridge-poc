// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use crate::bridges::{
	kusama_polkadot::{
		kusama_parachains_to_bridge_hub_polkadot::BridgeHubKusamaToBridgeHubPolkadotCliBridge,
		polkadot_parachains_to_bridge_hub_kusama::BridgeHubPolkadotToBridgeHubKusamaCliBridge,
	},
	rialto_parachain_millau::rialto_parachains_to_millau::RialtoParachainToMillauCliBridge,
	rococo_wococo::{
		rococo_parachains_to_bridge_hub_wococo::BridgeHubRococoToBridgeHubWococoCliBridge,
		wococo_parachains_to_bridge_hub_rococo::BridgeHubWococoToBridgeHubRococoCliBridge,
	},
	westend_millau::westend_parachains_to_millau::AssetHubWestendToMillauCliBridge,
};
use async_std::sync::Mutex;
use async_trait::async_trait;
use parachains_relay::parachains_loop::{AvailableHeader, SourceClient, TargetClient};
use relay_substrate_client::Parachain;
use relay_utils::metrics::{GlobalMetrics, StandaloneMetric};
use std::sync::Arc;
use structopt::StructOpt;
use strum::{EnumString, EnumVariantNames, VariantNames};
use substrate_relay_helper::{
	parachains::{source::ParachainsSource, target::ParachainsTarget, ParachainsPipelineAdapter},
	TransactionParams,
};

use crate::cli::{
	bridge::{CliBridgeBase, ParachainToRelayHeadersCliBridge},
	chain_schema::*,
	DefaultClient, PrometheusParams,
};

/// Start parachain heads relayer process.
#[derive(StructOpt)]
pub struct RelayParachains {
	/// A bridge instance to relay parachains heads for.
	#[structopt(possible_values = RelayParachainsBridge::VARIANTS, case_insensitive = true)]
	bridge: RelayParachainsBridge,
	#[structopt(flatten)]
	source: SourceConnectionParams,
	#[structopt(flatten)]
	target: TargetConnectionParams,
	#[structopt(flatten)]
	target_sign: TargetSigningParams,
	#[structopt(flatten)]
	prometheus_params: PrometheusParams,
}

/// Parachain heads relay bridge.
#[derive(Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "kebab_case")]
pub enum RelayParachainsBridge {
	RialtoToMillau,
	WestendToMillau,
	RococoToBridgeHubWococo,
	WococoToBridgeHubRococo,
	KusamaToBridgeHubPolkadot,
	PolkadotToBridgeHubKusama,
}

#[async_trait]
trait ParachainsRelayer: ParachainToRelayHeadersCliBridge
where
	ParachainsSource<Self::ParachainFinality, DefaultClient<Self::SourceRelay>>:
		SourceClient<ParachainsPipelineAdapter<Self::ParachainFinality>>,
	ParachainsTarget<Self::ParachainFinality, DefaultClient<Self::Target>>:
		TargetClient<ParachainsPipelineAdapter<Self::ParachainFinality>>,
	<Self as CliBridgeBase>::Source: Parachain,
{
	async fn relay_parachains(data: RelayParachains) -> anyhow::Result<()> {
		let source_client = data.source.into_client::<Self::SourceRelay>().await?;
		let source_client = ParachainsSource::<Self::ParachainFinality, _>::new(
			source_client,
			Arc::new(Mutex::new(AvailableHeader::Missing)),
		);

		let target_transaction_params = TransactionParams {
			signer: data.target_sign.to_keypair::<Self::Target>()?,
			mortality: data.target_sign.target_transactions_mortality,
		};
		let target_client = data.target.into_client::<Self::Target>().await?;
		let target_client = ParachainsTarget::<Self::ParachainFinality, _>::new(
			target_client.clone(),
			target_transaction_params,
		);

		let metrics_params: relay_utils::metrics::MetricsParams =
			data.prometheus_params.into_metrics_params()?;
		GlobalMetrics::new()?.register_and_spawn(&metrics_params.registry)?;

		parachains_relay::parachains_loop::run(
			source_client,
			target_client,
			metrics_params,
			futures::future::pending(),
		)
		.await
		.map_err(|e| anyhow::format_err!("{}", e))
	}
}

impl ParachainsRelayer for RialtoParachainToMillauCliBridge {}
impl ParachainsRelayer for AssetHubWestendToMillauCliBridge {}
impl ParachainsRelayer for BridgeHubRococoToBridgeHubWococoCliBridge {}
impl ParachainsRelayer for BridgeHubWococoToBridgeHubRococoCliBridge {}
impl ParachainsRelayer for BridgeHubKusamaToBridgeHubPolkadotCliBridge {}
impl ParachainsRelayer for BridgeHubPolkadotToBridgeHubKusamaCliBridge {}

impl RelayParachains {
	/// Run the command.
	pub async fn run(self) -> anyhow::Result<()> {
		match self.bridge {
			RelayParachainsBridge::RialtoToMillau =>
				RialtoParachainToMillauCliBridge::relay_parachains(self),
			RelayParachainsBridge::WestendToMillau =>
				AssetHubWestendToMillauCliBridge::relay_parachains(self),
			RelayParachainsBridge::RococoToBridgeHubWococo =>
				BridgeHubRococoToBridgeHubWococoCliBridge::relay_parachains(self),
			RelayParachainsBridge::WococoToBridgeHubRococo =>
				BridgeHubWococoToBridgeHubRococoCliBridge::relay_parachains(self),
			RelayParachainsBridge::KusamaToBridgeHubPolkadot =>
				BridgeHubKusamaToBridgeHubPolkadotCliBridge::relay_parachains(self),
			RelayParachainsBridge::PolkadotToBridgeHubKusama =>
				BridgeHubPolkadotToBridgeHubKusamaCliBridge::relay_parachains(self),
		}
		.await
	}
}
