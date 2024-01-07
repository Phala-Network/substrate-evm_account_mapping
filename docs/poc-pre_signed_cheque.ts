import { parse } from "https://deno.land/std/flags/mod.ts";
import { BN } from "https://deno.land/x/polkadot/util/mod.ts";
import { ApiPromise, HttpProvider, Keyring, WsProvider } from "https://deno.land/x/polkadot/api/mod.ts";
import { cryptoWaitReady } from "https://deno.land/x/polkadot/util-crypto/mod.ts";

const parsedArgs = parse(Deno.args, {
    alias: {
        "mnemonic": "m",
        "rpcUrl": "rpc-url",
    },
    string: [
        "mnemonic",
        "rpcUrl",
    ],
    default: {
        mnemonic: "safe potato popular make machine love horse quantum stuff pottery physical identify",
        rpcUrl: "ws://127.0.0.1:9944",
    },
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

await cryptoWaitReady().catch((e) => {
    console.error(e.message);
    Deno.exit(1);
});

const operatorKeyPair = (() => {
    const operatorMnemonic = parsedArgs.mnemonic.toString().trim();
    if (operatorMnemonic === undefined || operatorMnemonic === "") {
        return null;
    }

    try {
        return new Keyring({ type: "sr25519" }).addFromUri(operatorMnemonic, { name: "The migration operator" });
    } catch (e) {
        console.error(`Operator mnemonic invalid: ${e.message}`);
        return null;
    }
})();
if (operatorKeyPair === null) {
    console.error("Bad mnemonic");
    Deno.exit(1);
} else {
    console.log(`Operator: ${operatorKeyPair.address}`);
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

const cheque = api.createType("Cheque", {
    deadline: 10,
    sponsor_minimum_balance: numberToBalance(1),
    only_account: null,
    only_account_nonce: null,
    only_call_hash: null,
    sponsor_maximum_tip: numberToBalance(0),
})

const preSignedCheque = api.createType("PreSignedCheque", {
    cheque,
    signature: api.createType("SpRuntimeMultiSignature", {
        Sr25519: operatorKeyPair.sign(cheque.toU8a())
    }),
    signer: operatorKeyPair.address
})

console.log(preSignedCheque.toHex())

Deno.exit(0)
