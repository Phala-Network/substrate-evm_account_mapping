//! Benchmarking setup for the pallet
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as ThisPallet;
use frame_benchmarking::v2::*;
// use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	// impl_benchmark_test_suite!(ThisPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
