import { parse } from "https://deno.land/std/flags/mod.ts";
import { BN } from "https://deno.land/x/polkadot/util/mod.ts";
import { cryptoWaitReady, secp256k1PairFromSeed, encodeAddress, blake2AsU8a } from "https://deno.land/x/polkadot/util-crypto/mod.ts"
import { hexToU8a, u8aToHex } from "https://deno.land/x/polkadot/util/mod.ts"
import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import * as ethUtil from "npm:@ethereumjs/util";
import * as ethSigUtil from "npm:@metamask/eth-sig-util";
import { secp256k1 } from "npm:ethereum-cryptography/secp256k1.js"

const parsedArgs = parse(Deno.args, {
	alias: {
		"evmPrivateKey": "p",
		"sponsorMnemonic": "m",
		"rpcUrl": "rpc-url",
	},
	string: [
		"evmPrivateKey",
		"sponsorMnemonic",
		"rpcUrl",
	],
	default: {
		evmPrivateKey: "0x415ac5b1b9c3742f85f2536b1eb60a03bf64a590ea896b087182f9c92f41ea12",
		sponsorMnemonic: "safe potato popular make machine love horse quantum stuff pottery physical identify",
		rpcUrl: "ws://127.0.0.1:9944",
	},
});

await cryptoWaitReady().catch((e) => {
	console.error(e.message);
	Deno.exit(1);
});

function numberToBalance(value: BN | string | number) {
	const bn1e12 = new BN(10).pow(new BN(12));
	return new BN(value.toString()).mul(bn1e12);
}

function createSubstrateApi(rpcUrl: string): ApiPromise | null {
	let provider = null;
	if (rpcUrl.startsWith("wss://") || rpcUrl.startsWith("ws://")) {
		provider = new WsProvider(rpcUrl);
	} else if (rpcUrl.startsWith("https://") || rpcUrl.startsWith("http://")) {
		provider = new HttpProvider(rpcUrl);
	} else {
		return null;
	}

	return new ApiPromise({
		provider,
		throwOnConnect: true,
		throwOnUnknown: true,
		types: {
			Nonce: "u64",
			Cheque: {
				deadline: "BlockNumber",
				sponsor_minimum_balance: "Balance",
				only_account: "Option<AccountId>",
				only_account_nonce: "Option<Nonce>",
				only_call_hash: "Option<Hash>",
				sponsor_maximum_tip: "Balance",
			},
			PreSignedCheque: {
				cheque: "Cheque",
				signature: "SpRuntimeMultiSignature",
				signer: "AccountId",
			}
		}
	});
}

const sponsorKeyPair = (() => {
	const sponsorMnemonic = parsedArgs.sponsorMnemonic.toString().trim();
	if (sponsorMnemonic === undefined || sponsorMnemonic === "") {
		return null;
	}

	try {
		return new Keyring({ type: "sr25519" }).addFromUri(sponsorMnemonic, { name: "The migration sponsor" });
	} catch (e) {
		console.error(`sponsor mnemonic invalid: ${e.message}`);
		return null;
	}
})();
if (sponsorKeyPair === null) {
	console.error("Bad mnemonic");
	Deno.exit(1);
} else {
	console.log(`Sponsor: ${sponsorKeyPair.address}`);
}

const api = createSubstrateApi(parsedArgs.rpcUrl);
if (api === null) {
	console.error(`Invalid RPC URL "${parsedArgs.rpcUrl}"`);
	Deno.exit(1);
}

api.on("error", (e) => {
	console.error(`Polkadot.js error: ${e.message}"`);
	Deno.exit(1);
});

await api.isReady.catch((e) => console.error(e));

// Seed an ETH wallet

const ethPrivateKey = hexToU8a(parsedArgs.evmPrivateKey)
const ethPublicKey = ethUtil.privateToPublic(ethPrivateKey)
const ethCompressedPublicKey = secp256k1.getPublicKey(ethPrivateKey, true)
const ethEthAddress = ethUtil.privateToAddress(ethPrivateKey)

console.log(`ETH private key: ${u8aToHex(ethPrivateKey)}`)
console.log(`ETH public key: ${u8aToHex(ethPublicKey)}`)
console.log(`ETH compressed public key: ${u8aToHex(ethCompressedPublicKey)}`)
console.log(`ETH address: ${u8aToHex(ethEthAddress)}`)
console.log("")

// Map the ETH wallet to Sub wallet

const ss58Format = 42

console.log(`Substrate SS58 prefix: ${ss58Format}`)
console.log("")

const subKeyPair = function () {
	try {
		return secp256k1PairFromSeed(ethPrivateKey);
	} catch (e) {
		console.error(e.message)
		Deno.exit(1);
	}
}()

const subPublicKey = u8aToHex(subKeyPair.publicKey)
if (subPublicKey !== u8aToHex(ethCompressedPublicKey)) {
	console.error(`${subPublicKey} != ${u8aToHex(ethCompressedPublicKey)}`)
	Deno.exit(1)
}

const subKeyring = new Keyring({ type: "ecdsa", ss58Format })
const subKeyringPair = subKeyring.createFromPair(subKeyPair)
const subAddress = subKeyringPair.address

const subAddressFromPublicKey = encodeAddress(blake2AsU8a(ethCompressedPublicKey), ss58Format)
if (subAddress !== subAddressFromPublicKey) {
	console.error(`${subAddress} != 0x${subAddressFromPublicKey}`)
	Deno.exit(1)
}

// Prepare the meta call

const who = subAddressFromPublicKey
const call = api.tx.system.remarkWithEvent("Hello")
const callData = call.method.toHex()
const callHash = call.method.hash.toHex()
const nonce = 0
const tip = 0
const cheque = api.createType("Cheque", {
	deadline: 1000,
	sponsor_minimum_balance: numberToBalance(100),
	only_account: who,
	only_account_nonce: null,
	only_call_hash: callHash,
	sponsor_maximum_tip: numberToBalance(0),
})
const preSignedCheque = api.createType("PreSignedCheque", {
	cheque,
	signature: api.createType("SpRuntimeMultiSignature", {
		Sr25519: sponsorKeyPair.sign(cheque.toU8a())
	}),
	signer: sponsorKeyPair.address
}).toHex()

console.log("Meta call")
console.log(`Who: ${who}`)
console.log(`Call data: ${callData}`)
console.log(`Call hash: ${callHash}`)
console.log(`Nonce: ${nonce}`)
console.log(`Tip: ${tip}`)
console.log(`preSignedCheque: ${preSignedCheque}`)
console.log("")

// Prepare EIP-712 signature for the meta call

const eip712Name = "Substrate"
const eip712Version = "1"
const eip712ChainId = 0
const eip712VerifyingContract = "0x0000000000000000000000000000000000000000"

console.log("EIP-712 domain")
console.log(`Name: ${eip712Name}`)
console.log(`Version: ${eip712Version}`)
console.log(`ChainId: ${eip712ChainId}`)
console.log(`VerifyingContract: ${eip712VerifyingContract}`)
console.log("")

const eip712Data = {
	types: {
		EIP712Domain: [
			{
				name: "name",
				type: "string",
			},
			{
				name: "version",
				type: "string",
			},
			{
				name: "chainId",
				type: "uint256",
			},
			{
				name: "verifyingContract",
				type: "address",
			},
		],
		SubstrateCall: [
			{ name: 'who', type: 'string' },
			{ name: 'callData', type: 'bytes' },
			{ name: 'nonce', type: 'uint64' },
			{ name: 'tip', type: 'uint128' },
			{ name: 'preSignedCheque', type: 'bytes' },
		],
	},
	primaryType: "SubstrateCall",
	domain: {
		name: eip712Name,
		version: eip712Version,
		chainId: eip712ChainId,
		verifyingContract: eip712VerifyingContract,
	},
	message: {
		who,
		callData,
		nonce,
		tip,
		preSignedCheque
	},
}
const eip712Signature = ethSigUtil.signTypedData({
	privateKey: ethPrivateKey,
	data: eip712Data,
	version: ethSigUtil.SignTypedDataVersion.V3,
})
const eip712Hash = ethSigUtil.TypedDataUtils.eip712Hash(eip712Data, ethSigUtil.SignTypedDataVersion.V3);

console.log(`EIP-712 message: "${JSON.stringify(eip712Data)}"`)
console.log(`EIP-712 message hash: ${ethUtil.bytesToHex(eip712Hash)}`)
console.log(`EIP-712 signature: ${eip712Signature}`)
console.log("")

// Conclusion

console.log("evmAccountMapping.metaCall(who, call, nonce, tip, preSignedCheque, signature)")
const metaCall = api.tx.evmAccountMapping.metaCall(who, call, nonce, tip, preSignedCheque, eip712Signature)
console.log(`Call data: ${metaCall.method.toHex()}`)

Deno.exit(0)
