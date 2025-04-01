const csl = await import("@emurgo/cardano-serialization-lib-nodejs");
const f = await import('@mlabs-haskell/csl-gc-wrapper');
const lib = f.default(csl);

// Explicit re-exports are needed due to how nodejs processes CommonJS imports in ESM.
// If they are dynamic, they are only available via the `default` property of the module.
// Providing them explicitly lets nodejs analyze the module structure and put the
// appropriate entry points to the module interface.

// To re-generate the code use this snippet:

for (let key in lib) {
  if (Object.hasOwnProperty.bind(lib)(key)) {
    if (!key.startsWith('_') && key !== "default") {
      console.log('export const ' + key + ' = lib["'+key+'"];');
    }
  }
}
