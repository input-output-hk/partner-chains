{
  description = "Partner Chains - A Substrate-based blockchain with Cardano integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

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

  outputs = { self, nixpkgs, crane, fenix, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "SJwZ8g0zF2WrKDVmHrVG3pD2RGoQeo24MEXnNx5FyuI=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common build inputs for all targets
        commonArgs = {
          src = pkgs.lib.cleanSourceWith {
            src = self;
            filter = path: type:
              # Include all files that crane normally includes
              (craneLib.filterCargoSources path type) ||
              # Also include examples directories and JSON files
              (pkgs.lib.hasSuffix "examples" path) ||
              (pkgs.lib.hasSuffix ".json" path);
          };
          
          buildInputs = with pkgs; [
            openssl
            libclang.lib
          ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.rust-jemalloc-sys-unprefixed
          ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            pkgs.darwin.apple_sdk.frameworks.Security
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
            llvmPackages.lld
          ];

          # Environment variables
          CC_ENABLE_DEBUG_OUTPUT = "1";
          CRATE_CC_NO_DEFAULTS = 1;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            rustToolchain
            pkgs.stdenv.cc.cc
            pkgs.libz
            pkgs.clang
          ];
          
          BINDGEN_EXTRA_CLANG_ARGS = if pkgs.lib.hasSuffix "linux" system then "-I${pkgs.glibc.dev}/include -I${pkgs.clang.cc.lib}/lib/clang/19/include" else "";
          LIBCLANG_PATH = "${pkgs.clang.cc.lib}/lib";

          # Platform-specific flags
          CFLAGS = if pkgs.lib.hasSuffix "linux" system then
            "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
          else
            "";

          PROTOC = "${pkgs.protobuf}/bin/protoc";
          doCheck = false;
          #C_INCLUDE_PATH = "${pkgs.clang.cc.lib}/lib/clang/19/include";
          
          # RocksDB configuration - use system library
          ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
          OPENSSL_NO_VENDOR = 1;
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          RUSTFLAGS = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin
            "--cfg unwinding_backport --cfg unwinding_apple";

        };

        # Build the workspace dependencies
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "partner-chains-deps";
        });

        # Build the main binary
        partner-chains = craneLib.buildPackage (commonArgs // {
          pname = "partner-chains";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          
          inherit cargoArtifacts;

          # Git commit hash for Substrate CLI
          SUBSTRATE_CLI_GIT_COMMIT_HASH = self.dirtyShortRev or self.shortRev;
          
          postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            patchelf --set-rpath "${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/partner-chains-demo-node
          '';
        });

        # Run tests
        cargoTest = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });

        # Run clippy
        cargoClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

        # Run fmt check
        cargoFmt = craneLib.cargoFmt {
          inherit (commonArgs) src;
        };

      in
      {
        checks = {
          # Build the crate as part of `nix flake check`
          inherit partner-chains cargoTest cargoClippy cargoFmt;
        };

        packages = {
          default = partner-chains;
          inherit partner-chains;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
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
            patchelf
            pkg-config
            protobuf
            python312
            python312Packages.pip
            python312Packages.virtualenv
            rustToolchain
            sops
            xxd
          ] ++ (if pkgs.stdenv.hostPlatform.isDarwin then 
            [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ] 
          else 
            [ pkgs.clang ]);

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
            ${if pkgs.stdenv.hostPlatform.isLinux then "export CFLAGS=-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE" else ""}
          '';
        };

        formatter = pkgs.nixfmt-rfc-style;
      });

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
