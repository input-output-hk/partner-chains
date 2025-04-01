### csl-gc-wrapper

This is a small library that provides a wrapper for managing garbage collection for  [cardano-serialization-library](https://github.com/Emurgo/cardano-serialization-lib). It does this by using a FinalizationRegistry object to keep track of objects that are no longer in use, and calling the `free()` method on them when they are finalized.

Example usage:
```javascript
const csl = require("@emurgo/cardano-serialization-lib-browser");
const wrappedCsl = require('@mlabs-haskell/csl-gc-wrapper')(csl)

function fixture() {
  const arr = new Uint8Array(Array(10000000).fill(0));
  const pd = lib.PlutusData.new_bytes(arr);
}

setInterval(() => {
  for (let i = 0; i < 10; i++) {
    fixture()
  }
}, 500) // gc will trigger proxies and underlying pointers
```
