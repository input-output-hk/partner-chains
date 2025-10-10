{
  description = "Partner Chains - A Substrate-based blockchain with Cardano integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    
    crane.url = "github:ipetkov/crane";

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

        shellEnv = {
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

          CFLAGS = if pkgs.lib.hasSuffix "linux" system then
            "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
          else
            "";

          PROTOC = "${pkgs.protobuf}/bin/protoc";
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

        # Common build inputs for all targets
        commonArgs = {
          pname = "partner-chains-demo-node"; 
          src = pkgs.lib.cleanSourceWith {
            src = self;
            filter = path: type:
              (craneLib.filterCargoSources path type) ||
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

          doCheck = false;
        } // shellEnv;

        # Build the workspace dependencies separately
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "partner-chains-demo-node-deps";
          #cargoExtraArgs = "--workspace --all-targets --exclude substrate-test-runtime --exclude substrate-test-runtime-client";
        });

        partner-chains-demo-node = craneLib.buildPackage (commonArgs // {
          pname = "partner-chains-demo-node";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          
          inherit cargoArtifacts;

          # Git commit hash for partner-chains CLI --version flag
          SUBSTRATE_CLI_GIT_COMMIT_HASH = self.dirtyShortRev or self.shortRev;
          
          postFixup = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            patchelf --set-rpath "${pkgs.rocksdb}/lib:${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/partner-chains-demo-node
          '';
        });

        cargoTest = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });

        cargoClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          #cargoClippyExtraArgs = "--workspace --all-targets --exclude substrate-test-runtime --exclude substrate-test-runtime-client --exclude sc-network-test";
        });

        cargoFmt = craneLib.cargoFmt {
          inherit (commonArgs) pname src;
        };

      in
      {
        checks = { 
          # Build the crate as part of `nix flake check`
          inherit partner-chains-demo-node cargoTest # cargoClippy
            cargoFmt;
        };

        packages = {
          default = partner-chains-demo-node;
          inherit partner-chains-demo-node;
          ci = pkgs.runCommand "ci" {
            buildInputs = builtins.attrValues self.checks.${system};
          } "touch $out";
        };
        devShells.default = craneLib.devShell ({
          name = "partner-chains-demo-node-shell";
          # Inherit inputs from checks, which pulls in the build environment from packages.default (and others)
          checks = self.checks.${system};

          # Extra packages for the dev shell
          packages = with pkgs; [
            attic-client
            awscli2
            bashInteractive
            cargo-edit
            cargo-license
            coreutils
            docker-compose
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
            [pkgs.clang]);
        } // shellEnv);

        formatter = pkgs.nixfmt-rfc-style;
      });

  nixConfig = {
    allow-import-from-derivation = true;
    accept-flake-config = true;
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://cache.iog.io"
      "https://attic.sc.iog.io"
    ];
    extra-trusted-public-keys = [
      "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "partner-chains:j9StpxUY/znqFqaevhQRxCH4Hi0F4rCGXDiUSjz+kew="
    ];
  };
}
