use crate as pallet_commitments;

use frame_support::{
	pallet_prelude::ConstU32,
	parameter_types,
	traits::{ConstU128, ConstU16, ConstU64},
	BoundedVec,
};
use frame_system as system;
use pallet_balances::AccountData;

use sp_core::crypto::AccountId32;
pub use sp_core::{
	offchain::{testing as offchain_testing, OffchainWorkerExt, TransactionPoolExt},
	H256,
};
use sp_runtime::{
	testing::Header,
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
		Commitments: pallet_commitments
	}
);

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

type MaxParticipants = ConstU32<1>;

pub fn get_accounts() -> BoundedVec<AccountId32, MaxParticipants> {
	let accts = (0_u8..8_u8).map(|id| AccountId32::new([id; 32])).collect::<Vec<AccountId32>>();
	BoundedVec::try_from(accts).unwrap()
}

parameter_types! {
	pub const RevealWindowLength:u8 = 3;
}

impl pallet_commitments::Config for Test {
	type RevealWindowLength = RevealWindowLength;
	type MaxParticipants = MaxParticipants;
	type Hash = H256;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let mut ext = sp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}
