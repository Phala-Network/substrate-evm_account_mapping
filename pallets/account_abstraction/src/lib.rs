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
pub use weights::WeightInfo;

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
	traits::{tokens::Balance, Contains, OriginTrait},
	weights::{Weight, WeightMeter},
};
use sp_runtime::traits::{Convert, TrailingZeroInput};
use pallet_transaction_payment::ChargeTransactionPayment;

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::OnChargeTransaction<T>>::Balance;


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching call type.
		type RuntimeCall: Parameter
		+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
		+ GetDispatchInfo
		+ codec::Decode
		+ codec::Encode
		+ scale_info::TypeInfo
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
		EncodeError,
		DecodeError,
		NonceError,
		Overweight,
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
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::remote_call_from_evm_chain2 {
				ref account,
				ref call,
				ref nonce,
				ref signature,
			} = call {

			}

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
			let mut weight_counter = WeightMeter::max_limit();
			let mut origin: T::RuntimeOrigin = RawOrigin::Signed(account.clone()).into();
			origin.add_filter(T::CallFilter::contains);
			let call_result = Self::execute_dispatch(&mut weight_counter, origin, call);
			Self::deposit_event(Event::CallDone {
				who: account.clone(),
				call_result,
			});
			// TODO: deposit `weight_counter.consumed` from some where

			AccountNonce::<T>::insert(&account, current_nonce + 1);

			// TODO: need calculate the fee
			Ok(Pays::No.into())
		}

		/// Meta-transaction from EVM compatible chains
		#[pallet::call_index(1)]
		#[pallet::weight({0})]
		pub fn remote_call_from_evm_chain2(
			origin: OriginFor<T>,
			account: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
			nonce: u64,
			signature: [u8; 65]
		) -> DispatchResultWithPostInfo {
			use alloc::string::{String, ToString};
			use sp_io::hashing::{blake2_256, keccak_256};

			// This is an unsigned transaction
			ensure_none(origin)?;

			let current_nonce = AccountNonce::<T>::get(&account);
			ensure!(current_nonce == nonce, Error::<T>::NonceError);

			let call_data = <T as Config>::RuntimeCall::encode(&call);
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
			let mut weight_counter = WeightMeter::max_limit();
			let mut origin: T::RuntimeOrigin = RawOrigin::Signed(account.clone()).into();
			origin.add_filter(T::CallFilter::contains);
			let call_result = Self::execute_dispatch(&mut weight_counter, origin, call);
			Self::deposit_event(Event::CallDone {
				who: account.clone(),
				call_result,
			});
			// TODO: deposit `weight_counter.consumed` from some where

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

		/// Make a dispatch to the given `call` from the given `origin`, ensuring that the `weight`
		/// counter does not exceed its limit and that it is counted accurately (e.g. accounted using
		/// post info if available).
		///
		/// NOTE: Only the weight for this function will be counted (origin lookup, dispatch and the
		/// call itself).
		///
		/// Cheat from pallet-scheduler
		pub(crate) fn execute_dispatch(
			weight: &mut WeightMeter,
			origin: T::RuntimeOrigin,
			call: <T as Config>::RuntimeCall,
		) -> DispatchResult {
			let base_weight = Weight::zero();
			let call_weight = call.get_dispatch_info().weight;
			// We only allow a scheduled call if it cannot push the weight past the limit.
			let max_weight = base_weight.saturating_add(call_weight);

			if !weight.can_consume(max_weight) {
				return Err(Error::<T>::Overweight.into())
			}

			let (maybe_actual_call_weight, result) = match call.dispatch(origin) {
				Ok(post_info) => (post_info.actual_weight, Ok(())),
				Err(error_and_info) =>
					(error_and_info.post_info.actual_weight, Err(error_and_info.error)),
			};
			let call_weight = maybe_actual_call_weight.unwrap_or(call_weight);
			let _ = weight.try_consume(base_weight);
			let _ = weight.try_consume(call_weight);

			result.map(|_| ())
		}
	}
}
