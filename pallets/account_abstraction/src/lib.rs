#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::account_abstraction";

// Syntactic sugar for logging.
#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: $crate::LOG_TARGET,
			concat!("[{:?}] ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

use frame_support::{
	dispatch::{Dispatchable, PostDispatchInfo, GetDispatchInfo, RawOrigin},
	traits::{Contains, OriginTrait},
};
use sp_runtime::traits::TrailingZeroInput;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching call type.
		type RuntimeCall: Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ codec::Decode
			+ IsType<<Self as frame_system::Config>::RuntimeCall>;

		type CallFilter: Contains<<Self as frame_system::Config>::RuntimeCall>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A call just took place. \[result\]
		CallDone { call_result: DispatchResult },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		Unexpected,
		InvalidSignature,
		EthAddressMismatch,
		DecodeError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Meta-transaction from EVM compatible chains
		#[pallet::call_index(0)]
		#[pallet::weight({0})]
		pub fn remote_call_from_evm_chain(
			origin: OriginFor<T>,
			eth_address: [u8; 20],
			call_data: BoundedVec<u8, ConstU32<2048>>,
			signature: [u8; 65]
		) -> DispatchResultWithPostInfo {
			use alloc::string::{String, ToString};
			use frame_support::crypto::ecdsa::ECDSAExt;
			use sp_core::{blake2_256, keccak_256, ecdsa};

			// This is an unsigned transaction
			ensure_none(origin)?;

			let hexed_call_data = hex::encode(&call_data);
			let hexed_call_data_len_string = ((hexed_call_data.len() + 2) as u32).to_string();

			let mut eip191_message = String::from("\x19Ethereum Signed Message:\n");
			eip191_message.push_str(&hexed_call_data_len_string);
			eip191_message.push_str("0x");
			eip191_message.push_str(&hexed_call_data);

			let message_hash = keccak_256(eip191_message.as_bytes());
			let Some(recovered_public_key) = Self::ecdsa_recover_public_key(&signature, &message_hash) else {
				return Err(Error::<T>::InvalidSignature.into())
			};
			let public_key = ecdsa::Public::from_raw(recovered_public_key.serialize());
			let recovered_eth_address = public_key.to_eth_address().or(Err(Error::<T>::Unexpected))?;
			ensure!(recovered_eth_address == eth_address, Error::<T>::EthAddressMismatch);

			let raw_account = blake2_256(&public_key.0);
			let account = T::AccountId::decode(&mut &raw_account[..]).unwrap();
			let call = <T as Config>::RuntimeCall::decode(&mut TrailingZeroInput::new(&call_data)).or(Err(Error::<T>::DecodeError))?;

			let mut origin: T::RuntimeOrigin = RawOrigin::Signed(account).into();
			origin.add_filter(T::CallFilter::contains);
			let res = call.dispatch(origin);

			Self::deposit_event(Event::CallDone {
				call_result: res.map(|_| ()).map_err(|e| e.error),
			});

			// TODO: need calculate the fee
			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn ecdsa_recover_public_key(signature: &[u8; 65], message: &[u8; 32]) -> Option<secp256k1::PublicKey> {
			use secp256k1::{
				ecdsa::{RecoveryId, RecoverableSignature},
				Message, Secp256k1,
			};

			let rid = RecoveryId::from_i32(
				if signature[64] > 26 { signature[64] - 27 } else { signature[64] } as i32
			).ok()?;
			let sig = RecoverableSignature::from_compact(&signature[..64], rid).ok()?;
			let message = Message::from_slice(message).ok()?;

			Secp256k1::verification_only().recover_ecdsa(&message, &sig).ok()
		}
	}
}
