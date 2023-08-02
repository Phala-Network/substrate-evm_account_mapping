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
	dispatch::{Dispatchable, DispatchInfo, PostDispatchInfo, GetDispatchInfo, RawOrigin},
	traits::{Contains, OriginTrait},
	weights::{Weight, WeightMeter},
};
use sp_runtime::FixedPointOperand;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::traits::TrailingZeroInput;

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;

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
		+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, Info = DispatchInfo, PostInfo = PostDispatchInfo>
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
		CallDone { who: T::AccountId, call_result: DispatchResultWithPostInfo },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		Unexpected,
		InvalidSignature,
		AccountMismatch,
		DecodeError,
		NonceError,
		PaymentError,
	}

	#[pallet::storage]
	pub(crate) type AccountNonce<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		BalanceOf<T>: Send + Sync + FixedPointOperand,
		<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	{
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Only allow `remote_call_from_evm_chain`
			let Call::remote_call_from_evm_chain {
				ref who,
				ref call_data,
				ref nonce,
				ref signature,
			} = call else {
				return Err(InvalidTransaction::Call.into())
			};

			// Check nonce
			use sp_std::cmp::Ordering;
			let current_nonce = AccountNonce::<T>::get(&who);
			match nonce.cmp(&current_nonce) {
				Ordering::Greater => {
					return Err(InvalidTransaction::Future.into())
				},
				Ordering::Less => {
					return Err(InvalidTransaction::Stale.into())
				},
				_ => {}
			};

			// Validate the signature
			// TODO: Rewrite when implement EIP-712
			use alloc::string::{String, ToString};
			use sp_io::hashing::{blake2_256, keccak_256};

			let hexed_call_data = hex::encode(&call_data);
			let hexed_call_data_len = hexed_call_data.len() + 2;
			let nonce_len = nonce.to_string().len();

			let mut eip191_message = String::from("\x19Ethereum Signed Message:\n");
			eip191_message.push_str(&(hexed_call_data_len + nonce_len).to_string());
			eip191_message.push_str(&nonce.to_string());
			eip191_message.push_str("0x");
			eip191_message.push_str(&hexed_call_data);

			let message_hash = keccak_256(eip191_message.as_bytes());
			let Some(recovered_key) = Pallet::<T>::ecdsa_recover_public_key(signature, &message_hash) else {
				return Err(InvalidTransaction::BadProof.into())
			};

			// Validate the caller
			let public_key = recovered_key.to_encoded_point(true).to_bytes();
			let decoded_account = T::AccountId::decode(&mut &blake2_256(&public_key)[..]).unwrap();
			if who != &decoded_account {
				return Err(InvalidTransaction::BadSigner.into())
			}

			// Deserialize the call
			// TODO: Configurable upper bound?
			let actual_call = <T as Config>::RuntimeCall::decode(&mut TrailingZeroInput::new(call_data)).or(Err(InvalidTransaction::Call))?;

			// Withdraw the est fee
			// TODO: support tip
			let tip = 0u32.saturated_into::<BalanceOf<T>>();
			let len = actual_call.encoded_size();
			let info = actual_call.get_dispatch_info();
			// We shall get the same `fee` later
			// TODO: We can't get the exact fee for `Call::remote_call_from_evm_chain`, perhaps we can hard code a service fee?
			let est_fee = pallet_transaction_payment::Pallet::<T>::compute_fee(len as u32, &info, tip);
			let _ =
				<<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::withdraw_fee(who, &actual_call.into(), &info, est_fee, tip)?;

			// Calculate priority
			// Cheat from `get_priority` in frame/transaction-payment/src/lib.rs
			use sp_runtime::{traits::One, Saturating, SaturatedConversion};
			use frame_support::traits::Defensive;
			// Calculate how many such extrinsics we could fit into an empty block and take the
			// limiting factor.
			let max_block_weight = <T as frame_system::Config>::BlockWeights::get().max_block;
			let max_block_length = *<T as frame_system::Config>::BlockLength::get().max.get(info.class) as u64;

			// bounded_weight is used as a divisor later so we keep it non-zero.
			let bounded_weight = info.weight.max(Weight::from_parts(1, 1)).min(max_block_weight);
			let bounded_length = (len as u64).clamp(1, max_block_length);

			// returns the scarce resource, i.e. the one that is limiting the number of transactions.
			let max_tx_per_block_weight = max_block_weight
				.checked_div_per_component(&bounded_weight)
				.defensive_proof("bounded_weight is non-zero; qed")
				.unwrap_or(1);
			let max_tx_per_block_length = max_block_length / bounded_length;
			// Given our current knowledge this value is going to be in a reasonable range - i.e.
			// less than 10^9 (2^30), so multiplying by the `tip` value is unlikely to overflow the
			// balance type. We still use saturating ops obviously, but the point is to end up with some
			// `priority` distribution instead of having all transactions saturate the priority.
			let max_tx_per_block = max_tx_per_block_length
				.min(max_tx_per_block_weight)
				.saturated_into::<BalanceOf<T>>();
			let max_reward = |val: BalanceOf<T>| val.saturating_mul(max_tx_per_block);

			// To distribute no-tip transactions a little bit, we increase the tip value by one.
			// This means that given two transactions without a tip, smaller one will be preferred.
			let tip = tip.saturating_add(One::one());
			let scaled_tip = max_reward(tip);

			let priority = scaled_tip.saturated_into::<TransactionPriority>();

			ValidTransaction::with_tag_prefix("AccountAbstraction")
				// We set base priority to 2**20 and hope it's included before any
				// other transactions in the pool.
				.priority(priority)
				// This transaction does not require anything else to go before into
				// the pool. In theory we could require `previous_unsigned_at`
				// transaction to go first, but it's not necessary in our case.
				//.and_requires() We set the `provides` tag to be the same as
				// `next_unsigned_at`. This makes sure only one transaction produced
				// after `next_unsigned_at` will ever get to the transaction pool
				// and will end up in the block. We can still have multiple
				// transactions compete for the same "spot", and the one with higher
				// priority will replace other one in the pool.
				.and_provides(signature)
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
	impl<T: Config> Pallet<T>
	where
		BalanceOf<T>: FixedPointOperand,
		<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	{
		/// Meta-transaction from EVM compatible chains
		#[pallet::call_index(0)]
		#[pallet::weight({0})]
		pub fn remote_call_from_evm_chain(
			origin: OriginFor<T>,
			who: T::AccountId,
			call_data: BoundedVec<u8, ConstU32<2048>>,
			nonce: u64,
			signature: [u8; 65]
		) -> DispatchResultWithPostInfo {
			use alloc::string::{String, ToString};
			use sp_io::hashing::{blake2_256, keccak_256};

			// This is an unsigned transaction
			ensure_none(origin)?;

			let current_nonce = AccountNonce::<T>::get(&who);
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

			let decoded_account = T::AccountId::decode(&mut &blake2_256(&public_key)[..]).unwrap();
			ensure!(
				decoded_account == who,
				Error::<T>::AccountMismatch
			);

			let mut origin: T::RuntimeOrigin = RawOrigin::Signed(who.clone()).into();
			origin.add_filter(T::CallFilter::contains);
			let call = <T as Config>::RuntimeCall::decode(&mut TrailingZeroInput::new(&call_data)).or(Err(Error::<T>::DecodeError))?;
			let len = call.encoded_size();
			let info = call.get_dispatch_info();
			let call_result = call.dispatch(origin);
			let post_info = match call_result {
				Ok(post_info) => post_info,
				Err(error_and_info) => error_and_info.post_info,
			};

			Self::deposit_event(Event::CallDone {
				who: who.clone(),
				call_result,
			});

			// Should be the same as we withdrawn on `validate_unsigned`
			// TODO: support tip
			use sp_runtime::SaturatedConversion;
			let tip = 0u32.saturated_into::<BalanceOf<T>>();
			let est_fee = pallet_transaction_payment::Pallet::<T>::compute_fee(len as u32, &info, tip);

			let actual_fee = pallet_transaction_payment::Pallet::<T>::compute_actual_fee(len as u32, &info, &post_info, tip);
			// TODO: port the logic here
			// frame/transaction-payment/src/payment.rs
			let _ = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::correct_and_deposit_fee(
				&who, &info, &post_info, actual_fee, tip, Default::default()
			).map_err(|_err| Error::<T>::PaymentError)?;

			// TODO: Deposit `Event::<T>::TransactionFeePaid { who, actual_fee, tip }` event

			AccountNonce::<T>::insert(&who, current_nonce + 1);

			// TODO: need add the actual fee
			call_result
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
