EVM Account Mapping Pallet for Substrate
====

## Try

### Import ETH private key to Polkadot.js

> This is optional, only do it if you want to take full control of the wallet.

- You can get wallet's private key from "Account detail" on MetaMask
- In Polkadot.js app, click "+ Account"
  - Select "Raw seed", paste the private key to `seed (hex or string)`
    - You need to add `0x` for prefix
  - Expand "Advanced creation options" and select "ECDSA (Non BTC/ETH compatible)"
  - Check "I have saved ..." and click "Next"
- You can import the saved Json to the polkadot.js extension
  - There is no way to create an ECDSA wallet directly

Here's a seed private key for test:
`0x415ac5b1b9c3742f85f2536b1eb60a03bf64a590ea896b087182f9c92f41ea12`

### Run the dev node

```
$ cargo build --release
$ target/release/node-template --dev
```

### Understand how to make meta call data

`deno run --allow-all docs/poc.ts`

Here's a sample to send a call for `system.remarkWithEvent("Hello")`

```
evmAccountMapping.metaCall("5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF", system.remarkWithEvent("Hello"), 0, "0x37cb6ff8e296d7e476ee13a6cfababe788217519d428fcc723b482dc97cb4d1359a8d1c020fe3cebc1d06a67e61b1f0e296739cecacc640b0ba48e8a7555472e1b", None)
```

Before try this, transfer some token to `5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF`

## License

This project released under [Apache License, Version 2.0](https://opensource.org/license/apache-2-0/).
