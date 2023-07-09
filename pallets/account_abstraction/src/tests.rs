#[allow(unused)]
use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		let eth_address: [u8; 20] = hex::decode("e66bBB2B28273f4f0307e4c48fa30e304203016c").expect("Decodable").try_into().expect("Valid");
		let call_data = hex::decode("00071448656c6c6f").expect("Valid");
		let signature: [u8; 65] = hex::decode("ae6551cc19082ffa89c8c54b39282f00d214e477282ddea81b02908c78f08afc2c08474425f34ba54040a9eae1a592e3e3e60ee52b4ec88529b99ac29046f93b1c").expect("Decodable").try_into().expect("Valid");

		// Dispatch a signed extrinsic.
		assert_ok!(
			AccountAbstraction::remote_call_from_evm_chain(
				RuntimeOrigin::none(),
				eth_address,
				call_data.try_into().expect("Valid"),
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
// 	use sha3::{Keccak256, Digest};
//
// 	let eth_address: [u8; 20] = hex::decode("e66bBB2B28273f4f0307e4c48fa30e304203016c").expect("Decodable").try_into().expect("Valid");
// 	let call_data = "0x00071448656c6c6f";
// 	let signature: [u8; 65] = hex::decode("ae6551cc19082ffa89c8c54b39282f00d214e477282ddea81b02908c78f08afc2c08474425f34ba54040a9eae1a592e3e3e60ee52b4ec88529b99ac29046f93b1c").expect("Decodable").try_into().expect("Valid");
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
