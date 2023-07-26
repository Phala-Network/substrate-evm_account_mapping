#[allow(unused)]
use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::TrailingZeroInput;
use codec::Decode;

#[test]
fn it_works() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		let account = AccountId::from_ss58check("5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF").unwrap();
		let call_data = hex::decode("00071448656c6c6f").expect("Valid");
		let call = RuntimeCall::decode(&mut TrailingZeroInput::new(&call_data)).expect("Valid");
		let nonce: u64 = 0;
		let signature: [u8; 65] = hex::decode("8fe82b58127bdaf5090c00375181fb4152ec28af422e371d73a05b776c22f4e70aaa24e2d7604b65cfaf2fe332e6763c9cbafb59c1be7f4a0fd8cae1f3e351fb1b").expect("Decodable").try_into().expect("Valid");

		// Dispatch a signed extrinsic.
		// 0x07003d589a72aacea3f5f98494fdb5a7c3c70296b2410fa7552444d0206f61aa8e9100071448656c6c6f00000000000000008fe82b58127bdaf5090c00375181fb4152ec28af422e371d73a05b776c22f4e70aaa24e2d7604b65cfaf2fe332e6763c9cbafb59c1be7f4a0fd8cae1f3e351fb1b
		assert_ok!(
			AccountAbstraction::remote_call_from_evm_chain(
				RuntimeOrigin::none(),
				account,
				Box::<RuntimeCall>::new(call),
				nonce,
				signature
			)
		);

		// Assert that the correct event was deposited
		// System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

// #[test]
// fn foo() {
// 	use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
//
// 	let eth_address: [u8; 20] = hex::decode("e66bBB2B28273f4f0307e4c48fa30e304203016c").expect("Decodable").try_into().expect("Valid");
// 	let call_data = "00x00071448656c6c6f";
// 	let signature: [u8; 65] = hex::decode("8fe82b58127bdaf5090c00375181fb4152ec28af422e371d73a05b776c22f4e70aaa24e2d7604b65cfaf2fe332e6763c9cbafb59c1be7f4a0fd8cae1f3e351fb1b").expect("Decodable").try_into().expect("Valid");
//
// 	let eip191_message = format!("\x19Ethereum Signed Message:\n{}{}", call_data.len(), call_data);
// 	let message_hash = sp_core::keccak_256(eip191_message.as_bytes());
//
// 	let rid = RecoveryId::try_from(
// 		if signature[64] > 26 { signature[64] - 27 } else { signature[64] }
// 	).unwrap();
// 	let sig = Signature::from_slice(&signature[..64]).unwrap();
//
// 	let recovered_key = VerifyingKey::recover_from_prehash(
// 		&message_hash,
// 		&sig,
// 		rid
// 	).unwrap();
//
// 	let public_key = recovered_key.to_encoded_point(true);
// 	println!("0x{}", hex::encode(&public_key));
// 	let public_key = recovered_key.to_encoded_point(false);
// 	println!("0x{}", hex::encode(&public_key));
//
// 	let recovered_eth_address: [u8; 20] = sp_core::keccak_256(&recovered_key.to_encoded_point(false).as_bytes()[1..])[12..].try_into().unwrap();
// 	assert_eq!(recovered_eth_address, eth_address);
// }
