#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::keccak_256;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		InvalidSignature,
		EthAddressMismatch,
		Unexpected,
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
			// This is an unsigned transaction
			ensure_none(origin)?;

			let hexed_call_data = hex::encode(&call_data);
			let eip191_message = format!("\x19Ethereum Signed Message:\n{}0x{}", hexed_call_data.len() + 2, hexed_call_data);
			let message_hash = keccak_256(eip191_message.as_bytes());
			let Ok(recovered_pub_key) = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &message_hash) else {
				return Err(Error::<T>::InvalidSignature.into())
			};
			let recovered_eth_address: [u8; 20] = keccak_256(&recovered_pub_key)[12..].try_into().or(Err(Error::<T>::Unexpected))?;
			ensure!(recovered_eth_address == eth_address, Error::<T>::EthAddressMismatch);

			Ok(().into())
		}
	}
}
