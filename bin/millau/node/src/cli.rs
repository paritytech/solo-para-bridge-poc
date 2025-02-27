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

use clap::Parser;
// use sc_cli::RunCmd;
use std::fmt;

#[derive(clap::ValueEnum, Copy, Debug, Clone, PartialEq)]
pub enum NodeProcessingRole {
	LogicProvider,
	None,
}

impl fmt::Display for NodeProcessingRole {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl std::str::FromStr for NodeProcessingRole {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, String> {
		if s.eq_ignore_ascii_case("logic-provider") {
			Ok(Self::LogicProvider)
		} else if s.eq_ignore_ascii_case("none") {
			Ok(Self::None)
		} else {
			Err("Unknown string variant given for node-processing-role cli flag. Valid values are (logic-provider/none)".into())
		}
	}
}

#[derive(Debug, clap::Parser)]
pub struct RunCmd {
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,

	/// Run node as aggregator or logic provider.
	#[clap(long, value_enum, default_value_t = NodeProcessingRole::None)]
	pub node_processing_role: NodeProcessingRole,

	/// Set offchain config
	#[clap(long, value_enum)]
	pub set_config: Option<String>,
}

#[derive(Debug, Parser)]
pub struct Cli {
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[structopt(flatten)]
	pub run: RunCmd,
}

/// Possible subcommands of the main binary.
#[derive(Debug, Parser)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management CLI utilities
	#[clap(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Verify a signature for a message, provided on `STDIN`, with a given (public or secret) key.
	Verify(sc_cli::VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(sc_cli::VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(sc_cli::SignCmd),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Inspect blocks or extrinsics.
	Inspect(node_inspect::cli::InspectCmd),

	/// Benchmark runtime pallets.
	#[clap(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}
