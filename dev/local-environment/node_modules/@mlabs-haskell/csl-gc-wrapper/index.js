module.exports = (lib) => {
  if (lib.__gcPointerStore) { return lib }

  const finRegistry = new FinalizationRegistry((x) => {
    try {
      x.free()
    } catch (_) {}
  });

  const classWrap = (classObj) => {
    Object.getOwnPropertyNames(classObj).forEach((propName) => {
      if (propName === "__wrap") {
        const oldMethod = classObj[propName];
        classObj[propName] = function () {
          const retVal = oldMethod.apply(classObj, arguments);

          // Ensure that the returned object utilizes WASM memory by verifying
          // the presence of the ptr or __wbg_ptr property.
          //
          // Since ptr has been renamed to __wbg_ptr in wasm-bindgen v0.2.86,
          // we check for the presence of either variant to support all
          // CSL versions.
          // https://github.com/rustwasm/wasm-bindgen/pull/3408
          //
          // wasm_bindgen v0.2.83 -> v0.2.87 (cardano-serialization-lib v11.5.0)
          // https://github.com/Emurgo/cardano-serialization-lib/pull/632
          if (retVal && (retVal.__wbg_ptr || retVal.ptr)) {
            const px = new Proxy(retVal, {})
            finRegistry.register(px, retVal, px);
            return px
          }
          return retVal;
        };
      }
    });
    return classObj;
  };

  Object.keys(lib).forEach((k) => {
    if (k[0].toUpperCase() == k[0] && k[0] != "_") {
      classWrap(lib[k])
    }
  });

  lib.__gcPointerStore = finRegistry;
  return lib;
};
