import { cryptoWaitReady, secp256k1PairFromSeed, encodeAddress, blake2AsU8a } from "https://deno.land/x/polkadot/util-crypto/mod.ts"
import { Keyring } from "https://deno.land/x/polkadot/keyring/mod.ts"
import { hexToU8a, u8aToHex, stringToHex } from "https://deno.land/x/polkadot/util/mod.ts"
import EthCrypto from "npm:eth-crypto"

await cryptoWaitReady().catch((e) => {
  console.error(e.message);
  Deno.exit(1);
});

const ethPrivateKey = "415ac5b1b9c3742f85f2536b1eb60a03bf64a590ea896b087182f9c92f41ea12"
const ethPublicKey = EthCrypto.publicKeyByPrivateKey(`0x${ethPrivateKey}`)
const ethCompressedPublicKey = EthCrypto.publicKey.compress(ethPublicKey)
const ethAddress = EthCrypto.publicKey.toAddress(ethPublicKey)

// import * as ethers from "npm:ethers"
// const ethWallet = new ethers.Wallet(ethPrivateKey)
// console.log(`ETH address: ${ethWallet.address}`)

console.log(`ETH private key: 0x${ethPrivateKey}`)
console.log(`ETH public key: 0x${ethPublicKey}`)
console.log(`ETH compressed public key: 0x${ethCompressedPublicKey}`)
console.log(`ETH address: ${ethAddress}`)

const subKeyPair = function () {
  try {
    return secp256k1PairFromSeed(hexToU8a(`0x${ethPrivateKey}`));
  } catch (e) {
    console.error(e.message)
    Deno.exit(1);
  }
}()

const subPublicKey = u8aToHex(subKeyPair.publicKey)
if (subPublicKey !== `0x${ethCompressedPublicKey}`) {
  console.error(`${subPublicKey} != 0x${ethCompressedPublicKey}`)
  Deno.exit(1)
}

const subKeyring = new Keyring({ type: "ecdsa", ss58Format: 42 })
const subKeyringPair = subKeyring.createFromPair(subKeyPair)
const subAddress = subKeyringPair.address
console.log(`Sub address: ${subAddress}`)

const subAddressFromPublicKey = encodeAddress(blake2AsU8a(hexToU8a(`0x${ethCompressedPublicKey}`)), 42)
if (subAddress !== subAddressFromPublicKey) {
  console.error(`${subAddress} != 0x${subAddressFromPublicKey}`)
  Deno.exit(1)
}

const message = "0x00071448656c6c6f" // stringToHex("Hello world !!!")
console.log(`Message: ${message}`)

const hashedMessage = EthCrypto.hash.keccak256("\x19Ethereum Signed Message:\n" + message.length + message)
const signature = EthCrypto.sign(
    ethPrivateKey,
    hashedMessage
)
const r = signature.slice(0, 66);
const s = "0x" + signature.slice(66, 130);
const v = parseInt(signature.slice(130, 132), 16);
console.log(`EIP-191 message hash: ${hashedMessage}`)
console.log(`Message signature: ${signature}`)
console.log(`v: ${v} r: ${r} s: ${s}`)

// Won't work because computation of recovery bit (V) different
// console.log(secp256k1Verify(hexToU8a(hashedMessage), hexToU8a(signature), subKeyringPair.publicKey, "keccak", true))

import { secp256k1 } from "https://esm.sh/@noble/curves@1.1.0/secp256k1.js"
const hexedSignature = hexToU8a(signature)
const recovered = secp256k1.Signature
      .fromCompact(hexedSignature.subarray(0, 64))
      .addRecoveryBit(hexedSignature[64] - 27) // 0 or 1
      .recoverPublicKey(hexToU8a(hashedMessage))
      .toRawBytes();
if (u8aToHex(recovered) !== u8aToHex(subKeyringPair.publicKey)) {
  console.error(`${u8aToHex(recovered)} != ${u8aToHex(subKeyringPair.publicKey)}`)
  Deno.exit(1)
}

Deno.exit(0)
