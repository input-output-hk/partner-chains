Module generated.rs has been generated with cargo-typify from file ogmios.json.
File ogmios.json is in turn combination of files ogmios.json and cardano.json from https://github.com/CardanoSolutions/ogmios/tree/master/docs .
Recipe for the final file is:
* Copy content of `definitions` from `cardano.json` to `ogmios.json`
* remove ogmios.json or cardano.json from `$ref` references, because typify does not support them
* move few error definitions from `properties` to `definitions` in `ogmios.json`
