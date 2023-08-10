#[allow(unused)]
use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::TrailingZeroInput;
use codec::Decode;
use k256::ecdsa::signature;

#[test]
fn it_works() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		let account = AccountId::from_ss58check("5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF").unwrap();
		let call_data = hex::decode("00071448656c6c6f").expect("Valid");
		// let call = RuntimeCall::decode(&mut TrailingZeroInput::new(&call_data)).expect("Valid");
		let nonce: u64 = 0;
		let signature: [u8; 65] = hex::decode("37cb6ff8e296d7e476ee13a6cfababe788217519d428fcc723b482dc97cb4d1359a8d1c020fe3cebc1d06a67e61b1f0e296739cecacc640b0ba48e8a7555472e1b").expect("Decodable").try_into().expect("Valid");

		// TODO: this skip validate_unsigned so the nonce will mismatch
		// Dispatch a signed extrinsic.
		// 0x07003d589a72aacea3f5f98494fdb5a7c3c70296b2410fa7552444d0206f61aa8e9100071448656c6c6f00000000000000008fe82b58127bdaf5090c00375181fb4152ec28af422e371d73a05b776c22f4e70aaa24e2d7604b65cfaf2fe332e6763c9cbafb59c1be7f4a0fd8cae1f3e351fb1b
		assert_ok!(
			AccountAbstraction::remote_call_from_evm_chain(
				RuntimeOrigin::none(),
				account,
				call_data.try_into().expect("Valid"), // Box::<RuntimeCall>::new(call),
				nonce,
				signature
			)
		);

		// Assert that the correct event was deposited
		// System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

#[test]
fn eip712() {
	let eip712_name = b"Substrate".to_vec();
	let eip712_version = b"1".to_vec();
	let eip712_chain_id: crate::EIP712ChainID = sp_core::U256::from(0);
	let eip712_verifying_contract_address: crate::EIP712VerifyingContractAddress = TryInto::<[u8; 20]>::try_into(hex::decode("0000000000000000000000000000000000000000").expect("Decodable")).expect("Decodable").try_into().expect("Decodable");

	let eip712_domain = crate::eip712::EIP712Domain {
		name: eip712_name,
		version: eip712_version,
		chain_id: eip712_chain_id,
		verifying_contract: eip712_verifying_contract_address,
		salt: None,
	};
	let domain_separator = eip712_domain.separator();

	let type_hash = sp_io::hashing::keccak_256(
		"SubstrateCall(string who,bytes callData,uint64 nonce)".as_bytes(),
	);
	// Token::Uint(U256::from(keccak_256(&self.name)))
	let who = "5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF";
	let call_data = sp_io::hashing::keccak_256(&hex::decode("00071448656c6c6f").expect("Decodable"));
	let nonce = 0u64;
	let message_hash = sp_io::hashing::keccak_256(&ethabi::encode(&[
		ethabi::Token::FixedBytes(type_hash.to_vec()),
		ethabi::Token::FixedBytes(sp_io::hashing::keccak_256(who.as_bytes()).to_vec()),
		ethabi::Token::FixedBytes(call_data.to_vec()),
		ethabi::Token::Uint(nonce.into()),
	]));

	// panic!("{}", hex::encode(message_hash));

	let typed_data_hash_input = &vec![
		crate::encode::SolidityDataType::String("\x19\x01"),
		crate::encode::SolidityDataType::Bytes(&domain_separator),
		crate::encode::SolidityDataType::Bytes(&message_hash),
	];
	let bytes = crate::encode::abi::encode_packed(typed_data_hash_input);
	let signing_message = sp_io::hashing::keccak_256(bytes.as_slice());

	let signature: [u8; 65] = hex::decode("37cb6ff8e296d7e476ee13a6cfababe788217519d428fcc723b482dc97cb4d1359a8d1c020fe3cebc1d06a67e61b1f0e296739cecacc640b0ba48e8a7555472e1b").expect("Decodable").try_into().expect("Decodable");

	use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
	let rid = RecoveryId::try_from(
		if signature[64] > 26 { signature[64] - 27 } else { signature[64] }
	).unwrap();
	let sig = Signature::from_slice(&signature[..64]).unwrap();

	let recovered_key = VerifyingKey::recover_from_prehash(
		&signing_message,
		&sig,
		rid
	).unwrap();

	let public_key = recovered_key.to_encoded_point(true);
	println!("0x{}", hex::encode(&public_key));

	let decoded_account = AccountId::decode(&mut &sp_io::hashing::blake2_256(&public_key.to_bytes())[..]).expect("Decodable");
	assert_eq!(
		decoded_account.to_ss58check(),
		who
	);

	// let recovered_eth_public_key = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &signing_message).ok().expect("Recoverable");
	// // panic!("{}", hex::encode(recovered_eth_public_key));
	// let decoded_account = AccountId::decode(&mut &sp_io::hashing::blake2_256(&recovered_eth_public_key)[..]).expect("Decodable");
	// assert_eq!(
	// 	decoded_account.to_ss58check(),
	// 	who
	// );
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
