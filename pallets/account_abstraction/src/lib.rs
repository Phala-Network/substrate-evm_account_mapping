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
		CallDone { who: T::AccountId, call_result: DispatchResult },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		Unexpected,
		InvalidSignature,
		AccountMismatch,
		DecodeError,
		NonceError,
	}

	#[pallet::storage]
	pub(crate) type AccountNonce<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, _call: &Self::Call) -> TransactionValidity {
			ValidTransaction::with_tag_prefix("AccountAbstraction")
				// We set base priority to 2**20 and hope it's included before any
				// other transactions in the pool.
				.priority((1u64 << 20).into())
				// This transaction does not require anything else to go before into
				// the pool. In theory we could require `previous_unsigned_at`
				// transaction to go first, but it's not necessary in our case.
				//.and_requires() We set the `provides` tag to be the same as
				// `next_unsigned_at`. This makes sure only one transaction produced
				// after `next_unsigned_at` will ever get to the transaction pool
				// and will end up in the block. We can still have multiple
				// transactions compete for the same "spot", and the one with higher
				// priority will replace other one in the pool.
				.and_provides("my_tag")
				// The transaction is only valid for next 5 blocks. After that it's
				// going to be revalidated by the pool.
				.longevity(5)
				// It's fine to propagate that transaction to other peers, which
				// means it can be created even by nodes that don't produce blocks.
				// Note that sometimes it's better to keep it for yourself (if you
				// are the block producer), since for instance in some schemes
				// others may copy your solution and claim a reward.
				.propagate(true)
				.build()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Meta-transaction from EVM compatible chains
		#[pallet::call_index(0)]
		#[pallet::weight({0})]
		pub fn remote_call_from_evm_chain(
			origin: OriginFor<T>,
			account: T::AccountId,
			call_data: BoundedVec<u8, ConstU32<2048>>,
			nonce: u64,
			signature: [u8; 65]
		) -> DispatchResultWithPostInfo {
			use alloc::string::{String, ToString};
			use sp_io::hashing::{blake2_256, keccak_256};

			// This is an unsigned transaction
			ensure_none(origin)?;

			let current_nonce = AccountNonce::<T>::get(&account);
			ensure!(current_nonce == nonce, Error::<T>::NonceError);

			let hexed_call_data = hex::encode(&call_data);

			let hexed_call_data_len = hexed_call_data.len() + 2;
			let nonce_len = nonce.to_string().len();

			let mut eip191_message = String::from("\x19Ethereum Signed Message:\n");
			eip191_message.push_str(&(hexed_call_data_len + nonce_len).to_string());
			eip191_message.push_str(&nonce.to_string());
			eip191_message.push_str("0x");
			eip191_message.push_str(&hexed_call_data);

			let message_hash = keccak_256(eip191_message.as_bytes());
			let Some(recovered_key) = Self::ecdsa_recover_public_key(&signature, &message_hash) else {
				return Err(Error::<T>::InvalidSignature.into())
			};
			let public_key = recovered_key.to_encoded_point(true).to_bytes();

			// let raw_account = blake2_256(&public_key);
			let decoded_account = T::AccountId::decode(&mut &blake2_256(&public_key)[..]).unwrap();
			ensure!(
				decoded_account == account,
				Error::<T>::AccountMismatch
			);

			let call = <T as Config>::RuntimeCall::decode(&mut TrailingZeroInput::new(&call_data)).or(Err(Error::<T>::DecodeError))?;

			let mut origin: T::RuntimeOrigin = RawOrigin::Signed(account.clone()).into();
			origin.add_filter(T::CallFilter::contains);
			let res = call.dispatch(origin);

			Self::deposit_event(Event::CallDone {
				who: account.clone(),
				call_result: res.map(|_| ()).map_err(|e| e.error),
			});

			AccountNonce::<T>::insert(&account, current_nonce + 1);

			// TODO: need calculate the fee
			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn ecdsa_recover_public_key(signature: &[u8], message: &[u8]) -> Option<k256::ecdsa::VerifyingKey> {
			use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

			let rid = RecoveryId::try_from(
				if signature[64] > 26 { signature[64] - 27 } else { signature[64] }
			).ok()?;
			let sig = Signature::from_slice(&signature[..64]).ok()?;

			VerifyingKey::recover_from_prehash(
				message,
				&sig,
				rid
			).ok()
		}
	}
}
