#[allow(unused)]
use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		let eth_address: [u8; 20] = hex::decode("e66bBB2B28273f4f0307e4c48fa30e304203016c").expect("Decodable").try_into().expect("Valid");
		let call_data = "Hello world !!!";
		println!("{call_data}");
		let signature: [u8; 65] = hex::decode("838e8f298833f476bc871b175efcccba5c3cda88b1deab9f124aeed6cd095dea1ee3a0c88ae98ce7a3cccb5328db60bb7c9706f9c5593975d52f3ad4371ed92f1b").expect("Decodable").try_into().expect("Valid");

		// Dispatch a signed extrinsic.
		assert_ok!(
			AccountAbstraction::remote_call_from_evm_chain(
				RuntimeOrigin::none(),
				eth_address,
				call_data.as_bytes().to_vec().try_into().expect("Valid"),
				signature
			)
		);

		// Assert that the correct event was deposited
		// System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

// #[test]
// fn foo() {
// 	let eth_address: [u8; 20] = hex::decode("e66bBB2B28273f4f0307e4c48fa30e304203016c").expect("Decodable").try_into().expect("Valid");
// 	println!("{:?}", eth_address);
// 	let call_data = "0x48656c6c6f20776f726c6420212121";
// 	let signature: [u8; 65] = hex::decode("838e8f298833f476bc871b175efcccba5c3cda88b1deab9f124aeed6cd095dea1ee3a0c88ae98ce7a3cccb5328db60bb7c9706f9c5593975d52f3ad4371ed92f1b").expect("Decodable").try_into().expect("Valid");
//
// 	let eip191_message = format!("\x19Ethereum Signed Message:\n{}{}", call_data.len(), call_data);
// 	let message_hash = sp_core::keccak_256(eip191_message.as_bytes());
// 	let Ok(recovered_pub_key) = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &message_hash) else {
// 		panic!("Invalid signature")
// 	};
// 	println!("0x{}", hex::encode(&recovered_pub_key));
// 	println!("{:?}", recovered_pub_key);
// 	let recovered_eth_address: [u8; 20] = sp_core::keccak_256(&recovered_pub_key)[12..].try_into().unwrap();
// 	assert_eq!(recovered_eth_address, eth_address)
// }
