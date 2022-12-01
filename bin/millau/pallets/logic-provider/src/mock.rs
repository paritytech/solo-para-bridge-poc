use crate::{self as logic_provider, TemplateBridgedXcm};
use frame_support::{
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{ConstU128, ConstU16, ConstU64},
};
use frame_system as system;
use frame_system::EnsureRoot;
use pallet_balances::AccountData;

use sp_core::{crypto::AccountId32, Pair};
pub use sp_core::{
	offchain::{testing as offchain_testing, OffchainWorkerExt, TransactionPoolExt},
	H256,
};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Balances: pallet_balances,
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		LogicProvider: logic_provider::{Pallet, Call, Storage, Event<T>},
		Commitments: pallet_commitments,
	}
);

pub type Extrinsic = TestXt<Call, ()>;

impl<T> frame_system::offchain::SendTransactionTypes<T> for Test
where
	Call: From<T>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	/// The type for recording an account's balance.
	type Balance = u128;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = ConstU128<0>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

type MaxParticipants = ConstU32<255>;

parameter_types! {
	pub const FundsToLock: u128 = 10_000_000;
	pub const Reward: u128 = 1_000_000_000_000;
	pub const MaxCallPayloadLength: u16 = 300;
	pub const EnforceBurningTokens: bool = false;
}

pub struct MockBridging;
impl<Test: crate::Config> TemplateBridgedXcm<Test> for MockBridging {
	fn send_transact(
		_origin: system::pallet_prelude::OriginFor<Test>,
		_proof: Vec<u8>,
		_delivery_and_dispatch_fee: u64,
	) -> Result<([u8; 32], xcm::v3::MultiAssets), xcm::v3::SendError> {
		unimplemented!()
	}
}

impl logic_provider::Config for Test {
	type Event = Event;
	type MaxCallPayloadLength = MaxCallPayloadLength;
	type EnforceBurningTokens = EnforceBurningTokens;
	type Reward = Reward;
	type FundsToLock = FundsToLock;

	type ForceOrigin = EnsureRoot<AccountId32>;
	type WeightInfo = logic_provider::weights::SubstrateWeight<Self>;
	type LocalCurrency = Balances;
	type Bridging = MockBridging;
}

parameter_types! {
	pub const RevealWindowLength:u8 = 3;
}

impl pallet_commitments::Config for Test {
	type RevealWindowLength = RevealWindowLength;
	type MaxParticipants = MaxParticipants;
	type Hash = H256;
}

fn get_test_keys(len: usize) -> Vec<primitives::shared::Pair> {
	(0..len).map(|_| primitives::shared::Pair::generate().0).collect()
}

pub fn get_account_from_public(public: primitives::shared::Public) -> AccountId32 {
	let sp_public: sp_core::sr25519::Public = public.into();
	sp_public.into()
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> (sp_io::TestExternalities, Vec<primitives::shared::Pair>) {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let test_keys = get_test_keys(256);
	let public_keys = test_keys
		.iter()
		.map(|account| get_account_from_public(account.public()))
		.collect::<Vec<_>>();

	pallet_balances::GenesisConfig::<Test> {
		balances: public_keys.iter().cloned().map(|k| (k, 1 << 40)).collect(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	(ext, test_keys)
}
