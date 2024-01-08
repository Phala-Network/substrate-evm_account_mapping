import type { ApiPromise, SubmittableResult } from '@polkadot/api'
import type { ApiTypes, Signer as InjectedSigner } from '@polkadot/api/types'
import type { AddressOrPair, SubmittableExtrinsic } from '@polkadot/api-base/types/submittable'
import { encodeAddress } from '@polkadot/util-crypto'
import type { Account, Address, WalletClient } from 'viem'

import { recoverEvmPubkey, evmPublicKeyToSubstrateRawAddressU8a } from './addressConverter'
import { createEip712Domain, createSubstrateCall, createEip712StructedDataSubstrateCall } from './eip712'


export interface EtherAddressToSubstrateAddressOptions {
  SS58Prefix?: number
  msg?: string
}

export interface MappingAccount {
  evmAddress: Address
  substrateAddress: Address
  SS58Prefix: number
}

export async function getMappingAccount(
  api: ApiPromise,
  client: WalletClient,
  account: Account | { address: `0x${string}` },
  { SS58Prefix = 30, msg }: EtherAddressToSubstrateAddressOptions = {}
) {
    const version = api.consts.evmAccountMapping.eip712Version.toString()
    if (version !== '0x31' && version !== '0x32') {
      throw new Error(
        `Unsupported evm_account_mapping pallet version: consts.evmAccountMapping.eip712Version = ${version}`
      )
    }
    const recoveredPubkey = await recoverEvmPubkey(client, account as Account, msg)
    const converter = version === '0x32' ? 'EvmTransparentConverter' : 'SubstrateAddressConverter'
    const address = encodeAddress(
      evmPublicKeyToSubstrateRawAddressU8a(recoveredPubkey.compressed, converter),
      SS58Prefix
    )
  return {
    evmAddress: account.address,
    substrateAddress: address,
    SS58Prefix,
  } as MappingAccount
}


export class SignAndSendError extends Error {
  readonly isCancelled: boolean = false
}

export function callback<TSubmittableResult>(
  resolve: (value: TSubmittableResult) => void,
  reject: (reason?: any) => void,
  result: SubmittableResult,
  unsub?: any
) {
  if (result.status.isInBlock) {
    let error
    for (const e of result.events) {
      const {
        event: { data, method, section },
      } = e
      if (section === 'system' && method === 'ExtrinsicFailed') {
        error = data[0]
      }
    }

    if (unsub) {
      ;(unsub as any)()
    }
    if (error) {
      reject(error)
    } else {
      resolve(result as TSubmittableResult)
    }
  } else if (result.status.isInvalid) {
    ;(unsub as any)()
    reject('Invalid transaction')
  }
}

export function signAndSend<TSubmittableResult extends SubmittableResult = SubmittableResult>(
  target: SubmittableExtrinsic<ApiTypes, TSubmittableResult>,
  pair: AddressOrPair
): Promise<TSubmittableResult>
export function signAndSend<TSubmittableResult extends SubmittableResult = SubmittableResult>(
  target: SubmittableExtrinsic<ApiTypes, TSubmittableResult>,
  address: AddressOrPair,
  signer: InjectedSigner
): Promise<TSubmittableResult>
export function signAndSend<TSubmittableResult extends SubmittableResult = SubmittableResult>(
  target: SubmittableExtrinsic<ApiTypes, TSubmittableResult>,
  address: AddressOrPair,
  signer?: InjectedSigner
) {
  // Ready -> Broadcast -> InBlock -> Finalized
  return new Promise(async (resolve, reject) => {
    try {
      if (signer) {
        const unsub = await target.signAndSend(address, { signer }, (result) => {
          callback<TSubmittableResult>(resolve, reject, result, unsub)
        })
      } else {
        const unsub = await target.signAndSend(address, (result) => {
          callback<TSubmittableResult>(resolve, reject, result, unsub)
        })
      }
    } catch (error) {
      const isCancelled = (error as Error).message.indexOf('Cancelled') !== -1
      Object.defineProperty(error, 'isCancelled', {
        enumerable: false,
        value: isCancelled,
      })
      reject(error as SignAndSendError)
    }
  })
}

export async function signAndSendEvm<TSubmittableResult extends SubmittableResult = SubmittableResult>(
  extrinsic: SubmittableExtrinsic<'promise'>,
  apiPromise: ApiPromise,
  client: WalletClient,
  account: MappingAccount,
): Promise<TSubmittableResult> {
  const substrateCall = await createSubstrateCall(apiPromise, account.substrateAddress, extrinsic)
  const domain = createEip712Domain(apiPromise)
  const typedData = createEip712StructedDataSubstrateCall(domain, substrateCall)
  const signature = await client.signTypedData({ ...typedData, account: account.evmAddress })
  return await new Promise(async (resolve, reject) => {
    try {
      const _extrinsic = apiPromise.tx.evmAccountMapping.metaCall(
        account.substrateAddress,
        substrateCall.callData,
        substrateCall.nonce,
        signature,
        null
      )
      return _extrinsic.send((result) => callback(resolve, reject, result))
    } catch (error) {
      const isCancelled = (error as Error).message.indexOf('Cancelled') !== -1
      Object.defineProperty(error, 'isCancelled', {
        enumerable: false,
        value: isCancelled,
      })
      reject(error)
    }
  })
}
