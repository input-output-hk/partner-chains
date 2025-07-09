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

        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;
        customRustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in
      {
        packages.partner-chains = customRustPlatform.buildRustPackage rec {
        pname = "partner-chains";
        version = "1.7";
        src = ./.;
        preBuild = ''
          export SUBSTRATE_CLI_GIT_COMMIT_HASH=${self.dirtyShortRev or self.shortRev}
        '';

        useFetchCargoVendor = true;
        cargoHash = "sha256-QKBT22BsnBTCHZ8NuZPVxOnatPybZ+gt/9vg6HxNApc=";
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
              ]
            }"

            export OPENSSL_NO_VENDOR=1
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"

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
