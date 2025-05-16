{
  self,
  inputs,
  system,
  ...
}: {
  perSystem = {
    inputs',
    self',
    lib,
    pkgs,
    system,
    ...
  }: let
    flake-compat = import inputs.flake-compat;
    cardanoPackages = (flake-compat { src = inputs.cardano-node; }).defaultNix.packages.${system};
    dbSyncPackages = (flake-compat { src = inputs.cardano-dbsync; }).defaultNix.packages.${system};
    fenixPkgs = inputs'.fenix.packages;
    rustToolchain = with fenixPkgs;
      fromToolchainFile {
          file = ../../../rust-toolchain.toml;
          sha256 = "X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
        };
    customRustPlatform = pkgs.makeRustPlatform {
      cargo = rustToolchain;
      rustc = rustToolchain;
    };

  in {
    packages = {
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";
      ogmios = pkgs.callPackage ./ogmios.nix { };
      process-compose = pkgs.process-compose.overrideAttrs (oldAttrs: {
        patches = [ ./pc.patch ];
      });
      partnerchains-stack = pkgs.callPackage ./partnerchains-stack { inherit (self'.packages) partnerchains-stack-unwrapped; };
      partnerchains-stack-unwrapped = pkgs.callPackage ./partnerchains-stack-unwrapped { };
      partner-chains = customRustPlatform.buildRustPackage rec {
        pname = "partner-chains";
        version = "1.6";
        src = ../../../.;
        preBuild = ''
          export SUBSTRATE_CLI_GIT_COMMIT_HASH=${self.dirtyShortRev or self.shortRev}
        '';

        useFetchCargoVendor = true;
        cargoHash = "sha256-fV1CSccsvr30DiSaFrhjMHJlZ6OMxsvXdf5Sno7Nrb0=";
        buildType = "production";
        #buildAndTestSubdir = dir;
        doCheck = false;
        patches = [];

        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.protobuf

          pkgs.llvmPackages.lld
          customRustPlatform.bindgenHook
        ];
        buildInputs = [
          pkgs.rocksdb
          pkgs.openssl
          pkgs.libclang.lib
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
          pkgs.rust-jemalloc-sys-unprefixed
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          pkgs.darwin.apple_sdk.frameworks.Security
        ];

        postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
          patchelf --set-rpath "${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/partner-chains-demo-node
        '';

        # Force skip support check in CC crate
        #CRATE_CC_NO_DEFAULTS = "1";

        # Platform-specific features
        RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin
          "--cfg unwinding_backport --cfg unwinding_apple";

        # Existing environment variables
        CC_ENABLE_DEBUG_OUTPUT = "1";
        #CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.llvmPackages.lld}/bin/lld";
        #RUST_SRC_PATH = "${customRustPlatform.rustLibSrc}";
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
          rustToolchain
          pkgs.stdenv.cc.cc
          pkgs.libz
          pkgs.clang
        ];

        # Platform-specific flags
        CFLAGS =
          if pkgs.lib.hasSuffix "linux" system then
            "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
          else
            "";

        PROTOC = "${pkgs.protobuf}/bin/protoc";
        ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
        OPENSSL_NO_VENDOR = 1;
        OPENSSL_DIR = "${pkgs.openssl.dev}";
        OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.cc}/include -std=c++17";

      };
    };
  };
}
