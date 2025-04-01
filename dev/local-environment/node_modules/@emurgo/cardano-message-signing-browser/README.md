# Message Signing

This is a library that implements the [CIP-0008](https://github.com/cardano-foundation/CIPs/blob/master/CIP-0008/README.md) message signing spec for the Cardano blockchain.

The library is composed of structs for (de)serializing the CBOR defined in CIP-0008/COSE which lays at the core of the protocol, mostly defined in `lib.rs`, as well as many helper utilities for more specific cases useful to CIP-0008. These are mostly in `builders.rs` for building the CBOR structures specific to certain algorithms.

##### NPM packages

- [NodeJS WASM package](https://www.npmjs.com/package/@emurgo/cardano-message-signing-nodejs)
- [Browser (chrome/firefox) WASM package](https://www.npmjs.com/package/@emurgo/cardano-message-signing-browser)
- [Browser (pure JS - no WASM) ASM.js package](https://www.npmjs.com/package/@emurgo/cardano-message-signing-asmjs)


## Building

It can be used from both rust or compiled to wasm as all public code works with `wasm-bindgen` via `wasm-pack`.

There are no rust crates/npm packages uploaded yet, but these will come in the future.

In the meantime to build a wasm package we can run one of

* `npm run rust:build-nodejs` for nodejs targeted wasm
* `npm run rust:build-browser` for browser targeted wasm
* `npm run asm:build` for conversion for asm.js

and for use from rust simply use the lib that resides in `/rust/`.



## Example Usage

It is important to read the CIP-0008 spec to properly understand how to use this library. As per CIP-0008/COSE, signing is done via constructing a `SigStructure` and then signing this with the proper keys. This can be simplified via the use of the `COSESignBuilder` (for multiparty signing) / `COSESign1Builder` (for single signer) builders. Encryption is not yet supported by this library but will be in the future. An example node.js (wasm option) project that signs a message with a Cardano address exists in the `/example/` directory which has detailed comments describing each step.
