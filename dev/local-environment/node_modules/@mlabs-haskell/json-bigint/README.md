# json-bigint

This is a fork of [json-bigint](https://github.com/sidorares/json-bigint).

JSON.parse/stringify with bigints support. Based on Douglas Crockford [JSON.js](https://github.com/douglascrockford/JSON-js) package and [bignumber.js](https://github.com/MikeMcl/bignumber.js) library.

This is a simplified fork with removed config options. Native `BigInt` is always used for integers, and numbers with decimals are represented by `Number`s.
