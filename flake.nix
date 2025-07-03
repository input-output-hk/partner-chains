{
  description = "Your devShell environment using flake-utils";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    cardano-node = {
      url = "github:IntersectMBO/cardano-node/10.1.4";
      flake = false;
    };

    flake-compat = {
      url = "github:input-output-hk/flake-compat/fixes";
      flake = false;
    };
  };

  outputs =
    { 
      self,
      nixpkgs,
      fenix,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
        };

        # Exact commit behind the polkadot-stable2503-5 tag.
        polkadot-sdk-src = pkgs.fetchgit {
          url = "https://github.com/paritytech/polkadot-sdk.git";
          rev = "f4dba53b456ab6f16e6b7b6cc0e12b0061d4238e"; # polkadot-stable2503-5
          hash = "sha256-Yt0KWRMOG53hxdMZvYA60hQ4Vsfkk1R5lv+dd+mzcNI=";
        };        
        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;
        customRustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Define cargo path for use in overrides
        cargo = "${rustToolchain}/bin/cargo";

        # Import the crate2nix generated Cargo.nix
        cargoNix = pkgs.callPackage ./Cargo.nix {
          # Override buildRustCrate to use our updated Rust toolchain
          buildRustCrateForPkgs = pkgs: (pkgs.buildRustCrate.override {
            rustc = rustToolchain;
            cargo = rustToolchain;
          });
          # Override defaultCrateOverrides to fix problematic packages
          defaultCrateOverrides = pkgs.defaultCrateOverrides // {
            # Override wasm-opt-sys - use proper C++17 support
            wasm-opt-sys = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.cmake
              ];
              buildInputs = (attrs.buildInputs or []) ++ [
                pkgs.binaryen
                pkgs.libclang.lib
              ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                pkgs.darwin.apple_sdk.frameworks.Security
              ];
              
              # Use proper C++17 with better libc++ support
              CC_ENABLE_DEBUG_OUTPUT = "1";
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
              
              # Skip broken-symlink check – the crate creates a dangling
              # cxxbridge header link that is harmless at runtime but breaks
              # nix\'s fixupPhase.
              dontCheckForBrokenSymlinks = true;

              # As an extra safeguard, drop any dangling links that might have
              # been produced during compilation.
              postFixup = ''
                find $out -xtype l -delete || true
              '';
            };
            # working
            wasm-opt-cxx-sys = attrs: let
              binaryen116 = pkgs.fetchFromGitHub {
              owner  = "WebAssembly";
              repo   = "binaryen";
              rev    = "version_116";         # exact tag for 116
              hash   = "sha256-gMwbWiP+YDCVafQMBWhTuJGWmkYtnhEdn/oofKaUT08=";   # <-- fill with nix hash
            };
          in {
              CARGO = cargo;
              LIBCLANG_PATH = "${pkgs.clang.cc.lib}/lib";
              patchPhase = ''
                cp -r ${binaryen116} binaryen
              '';
              CXXFLAGS = "-I${binaryen116}/src -I${binaryen116}/src/tools";
              nativeBuildInputs = [
                pkgs.clang
                pkgs.llvm
                pkgs.pkg-config
                #pkgs.breakpointHook
              ];
              dontCheckForBrokenSymlinks = true;
            };
            # Override rocksdb-related packages
            librocksdb-sys = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.llvmPackages.clang-unwrapped.lib
                #pkgs.breakpointHook
              ];
              buildInputs = (attrs.buildInputs or []) ++ [
                pkgs.rocksdb
                pkgs.pkg-config
              ];
              ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
              RUST_BACKTRACE="full";
              LIBCLANG_PATH = "${pkgs.clang.cc.lib}/lib";
              C_INCLUDE_PATH = "${pkgs.clang.cc.lib}/lib/clang/19/include";
              # Ensure bindgen can locate standard C headers like stdarg.h
              BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include -I${pkgs.clang.cc.lib}/lib/clang/19/include";
            };

            # Override openssl-related packages
            openssl-sys = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [
                pkgs.pkg-config
              ];
              buildInputs = (attrs.buildInputs or []) ++ [
                pkgs.openssl
              ];
              OPENSSL_NO_VENDOR = "1";
              OPENSSL_DIR = "${pkgs.openssl.dev}";
              OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
              OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
            };

            # Override protobuf-related packages
            prost-build = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [
                pkgs.protobuf
              ];
              PROTOC = "${pkgs.protobuf}/bin/protoc";
            };

            # Override proc-macro-crate to bypass problematic nixpkgs patches
            proc-macro-crate = attrs: attrs // {
              # Remove any problematic patches or substitutions
              patches = [];
              prePatch = "";
              postPatch = "";
              # Don't do any pattern substitutions that might fail
              doCheck = false;
            };

            # Fix parity-scale-codec dependency resolution for substrate crates
            # The derive macros need CARGO to run cargo metadata to find dependencies
            binary-merkle-tree = attrs: { CARGO = cargo; };
            fork-tree = attrs: { CARGO = cargo; };
            scale-info = attrs: { CARGO = cargo; };
            sp-storage = attrs: { CARGO = cargo; };
            sp-tracing = attrs: { CARGO = cargo; };
            sp-version-proc-macro = attrs: { CARGO = cargo; };
            sp-externalities = attrs: { CARGO = cargo; };
            bounded-collections = attrs: { CARGO = cargo; };
            frame-metadata = attrs: { CARGO = cargo; };
            finality-grandpa = attrs: { CARGO = cargo; };
            primitive-types = attrs: { CARGO = cargo; };
            sp-arithmetic = attrs: { CARGO = cargo; };
            sp-metadata-ir = attrs: { CARGO = cargo; };
            sp-weights = attrs: { CARGO = cargo; };
            sp-wasm-interface = attrs: { CARGO = cargo; };
            sp-core = attrs: { CARGO = cargo; };
            sc-state-db = attrs: { CARGO = cargo; };
            sp-trie = attrs: { CARGO = cargo; };
            sp-io = attrs: { CARGO = cargo; };
            sp-application-crypto = attrs: { CARGO = cargo; };
            sp-runtime = attrs: { CARGO = cargo; };
            sp-staking = attrs: { CARGO = cargo; };
            sp-version = attrs: { CARGO = cargo; };
            sp-inherents = attrs: { CARGO = cargo; };
            sp-api = attrs: { CARGO = cargo; };
            sp-timestamp = attrs: { CARGO = cargo; };
            sp-transaction-storage-proof = attrs: { CARGO = cargo; };
            sp-session-validator = attrs: { CARGO = cargo; };
            sp-session-validator-management = attrs: { CARGO = cargo; };
            frame-system-rpc-runtime-api = attrs: { CARGO = cargo; };
            sp-block-builder = attrs: { CARGO = cargo; };
            sp-blockchain = attrs: { CARGO = cargo; };
            sp-genesis-builder = attrs: { CARGO = cargo; };
            sp-consensus-slots = attrs: { CARGO = cargo; };
            sp-consensus-grandpa = attrs: { CARGO = cargo; };
            sp-mixnet = attrs: { CARGO = cargo; };
            sp-mmr-primitives = attrs: { CARGO = cargo; };
            sp-consensus-beefy = attrs: { CARGO = cargo; };
            sp-offchain = attrs: { CARGO = cargo; };
            sp-session = attrs: { CARGO = cargo; };
            sp-transaction-pool = attrs: { CARGO = cargo; };
            sp-statement-store = attrs: { CARGO = cargo; };
            frame-support = attrs: { CARGO = cargo; };
            sp-consensus-aura = attrs: { CARGO = cargo; };
            authority-selection-inherents = attrs: { CARGO = cargo; };
            sp-sidechain = attrs: { CARGO = cargo; };
            frame-system = attrs: { CARGO = cargo; };
            pallet-sidechain-rpc = attrs: { CARGO = cargo; };
            pallet-mmr = attrs: { CARGO = cargo; };          
            pallet-beefy = attrs: { CARGO = cargo; };
            pallet-beefy-mmr = attrs: { CARGO = cargo; };
            sp-session-validator-management-query = attrs: { CARGO = cargo; };
            pallet-session-validator-management-rpc = attrs: { CARGO = cargo; };
            pallet-block-rewards = attrs: { CARGO = cargo; };
            pallet-native-token-management = attrs: { CARGO = cargo; };
            pallet-session-validator-management = attrs: { CARGO = cargo; };
            pallet-sidechain = attrs: { CARGO = cargo; };
            pallet-sudo = attrs: { CARGO = cargo; };
            pallet-transaction-payment = attrs: { CARGO = cargo; };
            pallet-transaction-payment-rpc = attrs: { CARGO = cargo; };
            pallet-partner-chains-session = attrs: { CARGO = cargo; };
            pallet-grandpa = attrs: { CARGO = cargo; };
            pallet-aura = attrs: { CARGO = cargo; };
            pallet-balances = attrs: { CARGO = cargo; };
            pallet-transaction-payment-rpc-runtime-api = attrs: { CARGO = cargo; };
            pallet-authorship = attrs: { CARGO = cargo; };
            pallet-timestamp = attrs: { CARGO = cargo; };
            frame-benchmarking = attrs: { CARGO = cargo; };
            pallet-session = attrs: { CARGO = cargo; };
            sc-client-db = attrs: { CARGO = cargo; };
            sc-network-common = attrs: { CARGO = cargo; };
            sc-mixnet = attrs: { CARGO = cargo; };
            sc-rpc-api = attrs: { CARGO = cargo; };
            sc-consensus-grandpa = attrs: { CARGO = cargo; };
            sc-consensus-beefy = attrs: { CARGO = cargo; };
            substrate-frame-rpc-system = attrs: { CARGO = cargo; };
            sc-consensus-grandpa-rpc = attrs: { CARGO = cargo; };
            sc-rpc-spec-v2 = attrs: { CARGO = cargo; };
            sc-cli = attrs: { CARGO = cargo; };
            sc-network = attrs: { 
              CARGO = cargo;
              buildInputs = [pkgs.protobuf];
            };
            sc-network-light = attrs: {
              buildInputs = [pkgs.protobuf];
            };
            sc-network-sync = attrs: { 
              CARGO = cargo;
              buildInputs = [pkgs.protobuf];
            };
            litep2p = attrs: {
              buildInputs = [pkgs.protobuf];
            };

            # Fix arkworks crates dependency resolution with CARGO overrides
            # Just disable checks and provide CARGO - let them build if possible
            ark-poly = attrs: { CARGO = cargo; doCheck = false; };
            ark-ec = attrs: { CARGO = cargo; doCheck = false; };
            ark-ff = attrs: { CARGO = cargo; doCheck = false; };
            ark-serialize = attrs: { CARGO = cargo; doCheck = false; };
            ark-std = attrs: { CARGO = cargo; doCheck = false; };
            ark-bls12-381 = attrs: { CARGO = cargo; doCheck = false; };
            ark-ed-on-bls12-381 = attrs: { CARGO = cargo; doCheck = false; };
            ahash = attrs: { CARGO = cargo; doCheck = false; };
            educe = attrs: { CARGO = cargo; doCheck = false; };
            partner-chains-demo-runtime = attrs: { 
              CARGO = cargo; 
              doCheck = false; 
              CARGO_MANIFEST_DIR = "${self}";
              WASM_BUILD_WORKSPACE_HINT = "${self}";
              CARGO_NET_OFFLINE = "true";
              nativeBuildInputs = [pkgs.breakpointHook];
#               patchPhase = ''
#                 echo ${builtins.toJSON attrs}
#                 echo $WASM_BUILD_WORKSPACE_HINT
#                 mkdir -p .cargo
#                 cat > .cargo/config.toml <<EOF
# [source."https://github.com/paritytech/polkadot-sdk.git"]
# git = "https://github.com/paritytech/polkadot-sdk.git"
# replace-with = "polkadot-sdk-local"

# [source.polkadot-sdk-local]
# directory = "${polkadot-sdk-src}"
# EOF
#               '';
            };

            # Override main demo node package with working environment variables
            partner-chains-demo-node = attrs: {
              nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [
                pkgs.pkg-config
                pkgs.protobuf
                pkgs.llvmPackages.lld
                customRustPlatform.bindgenHook
              ];
              buildInputs = (attrs.buildInputs or []) ++ [
                pkgs.rocksdb
                pkgs.openssl
                pkgs.libclang.lib
              ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
                pkgs.rust-jemalloc-sys-unprefixed
              ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                pkgs.darwin.apple_sdk.frameworks.Security
              ];

              # Environment variables from your working build
              CC_ENABLE_DEBUG_OUTPUT = "1";
              CARGO_TARGET_WASM32V1_NONE_LINKER = "${pkgs.llvmPackages.lld}/bin/lld";
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
              LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
                rustToolchain
                pkgs.stdenv.cc.cc
                pkgs.libz
                pkgs.clang
              ];

              # Platform-specific flags from your working build
              CFLAGS = if pkgs.lib.hasSuffix "linux" system then
                "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
              else
                "";

              # Force skip support check in CC crate (from your working build)
              CRATE_CC_NO_DEFAULTS = "1";

              PROTOC = "${pkgs.protobuf}/bin/protoc";
              ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
              OPENSSL_NO_VENDOR = "1";
              OPENSSL_DIR = "${pkgs.openssl.dev}";
              OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
              OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
              BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.stdenv.cc.cc}/include -std=c++17";

              # Platform-specific Rust flags from your working build
              RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin
                "--cfg unwinding_backport --cfg unwinding_apple";

              # Post-build fixup for Linux from your working build
              postInstall = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
                patchelf --set-rpath "${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/partner-chains-demo-node
              '';
            };
          };
        };

      in
      {
        packages.partner-chains = cargoNix.workspaceMembers.partner-chains-demo-node.build;

        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              awscli2
              bashInteractive
              cargo-edit
              cargo-license
              coreutils
              docker-compose
              earthly
              gawk
              gnumake
              kubectl
              libiconv
              nixfmt-rfc-style
              openssl
              pkg-config
              protobuf
              python312
              python312Packages.pip
              python312Packages.virtualenv
              rustToolchain
              sops
            ]
            ++ (if isDarwin then [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ] else [ pkgs.clang ]);

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export LD_LIBRARY_PATH="${
              pkgs.lib.makeLibraryPath [
                rustToolchain
                pkgs.libz
                pkgs.stdenv.cc.cc
              ]
            }"

            export OPENSSL_NO_VENDOR=1
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"

            export PYTHONNOUSERSITE=1
            export CRATE_CC_NO_DEFAULTS=1
            ${if isLinux then "export CFLAGS=-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE" else ""}
          '';
        };
        formatter = pkgs.nixfmt-rfc-style;
      }
    );

  nixConfig = {
    allow-import-from-derivation = true;
    accept-flake-config = true;
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://cache.iog.io"
      "https://cache.sc.iog.io"
    ];
    extra-trusted-public-keys = [
      "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "cache.sc.iog.io:b4YIcBabCEVKrLQgGW8Fylz4W8IvvfzRc+hy0idqrWU="
    ];
  };
}
