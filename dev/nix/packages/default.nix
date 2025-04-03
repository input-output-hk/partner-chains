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
    # rustToolchain = with fenixPkgs;
    #   fromToolchainFile {
    #       file = ../../../rust-toolchain.toml;
    #       sha256 = "VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
    #     }
    rustToolchain = fenixPkgs.combine [
      fenixPkgs.latest.toolchain
      fenixPkgs.latest.rust-src
      fenixPkgs.targets.wasm32-unknown-unknown.latest.rust-std
    ];
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
      partner-chains = 
      let 
        # Create a patched source
        patchedSrc = pkgs.stdenv.mkDerivation {
          name = "partner-chains-patched-src";
          src = ../../../.;
          patches = [./rust-src-std.patch];
          buildPhase = "true"; # Skip build
          installPhase = ''
            cp -r . $out
          '';
        };
      in customRustPlatform.buildRustPackage rec {
        pname = "partner-chains";
        version = "1.5";
        src = patchedSrc; # Use the pre-patched source
        
        cargoLock = {
          lockFile = "${patchedSrc}/Cargo.lock"; # Point to the patched lock file
          outputHashes = {
            "binary-merkle-tree-16.0.0" = "sha256-E7Mq/0EwqVSnJ6nX7TZppLjwje7vYuxjjCkt5kWQjfQ=";
            "pallas-addresses-0.31.0" = "sha256-gecokNSH018NSFb8Cm+Gvql8wkip0t/IkdgY3ZRILbE=";
            "raw-scripts-7.0.2" = "sha256-rUms2AZGOEjh1/zvfVoqv/B1XFUiglDf9UAMaqFIQZU=";
          };
        };
        
        doCheck = false;
        
        # We no longer need to patch here, as we're using pre-patched source
        patches = [];
        
        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.clang
          pkgs.llvmPackages.lld
          customRustPlatform.bindgenHook
        ];
        
        buildInputs = [
          pkgs.rocksdb
          pkgs.openssl
          pkgs.libclang.lib
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.rust-jemalloc-sys-unprefixed ]
        ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
          pkgs.Security
          pkgs.SystemConfiguration
        ];
        
        postFixup = ''
          patchelf --set-rpath ${pkgs.rocksdb}/lib $out/bin/partner-chains-node
        '';
        
        CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.llvmPackages.lld}/bin/lld";
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
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
        BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.cc}/include";
        
        # Add flags for building std library
        #RUSTFLAGS = "-Z build-std";
      };
    };
  };
}
