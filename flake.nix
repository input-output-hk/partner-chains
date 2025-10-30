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

    n2c = {
      url = "github:nlewo/nix2container";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, n2c, ... }:
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
          
          BINDGEN_EXTRA_CLANG_ARGS = if pkgs.lib.hasSuffix "linux" system
                                     then "-I${pkgs.glibc.dev}/include -I${pkgs.clang.cc.lib}/lib/clang/19/include"
                                     else "";
          LIBCLANG_PATH = "${pkgs.clang.cc.lib}/lib";

          CFLAGS = if pkgs.lib.hasSuffix "linux" system then
            "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
          else
            "";

          PROTOC = "${pkgs.protobuf}/bin/protoc";
          #C_INCLUDE_PATH = "${pkgs.clang.cc.lib}/lib/clang/19/include";

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
          # Clean the project directory so that the nix hash
          # doesn't change when unrelated files to builds update
          src = let
            jsonFilter = path: _type: builtins.match ".*\\.json$" path != null;
            combinedFilter = path: type:
              (craneLib.filterCargoSources path type) ||
              (jsonFilter path type);
          in pkgs.lib.cleanSourceWith {
            src = self;
            filter = combinedFilter;
            name = "source";
          };
          
          buildInputs = with pkgs; [
            openssl
            libclang.lib
            stdenv.cc.cc.lib
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
            autoPatchelfHook
          ];

          doCheck = false;
        } // shellEnv;

        cargoVendorDir = craneLib.vendorCargoDeps {
          inherit (commonArgs) src;
          # Remove fixture and example directories from Polkadot SDK, don't want them vendored/checked
          # https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2412-1/.github/workflows/checks-quick.yml#L91-L97
          overrideVendorGitCheckout = let
            isPolkadotSdk = p: pkgs.lib.hasPrefix "git+https://github.com/paritytech/polkadot-sdk.git" p.source;
          in ps: drv:
            if pkgs.lib.any (p: isPolkadotSdk p) ps then
              drv.overrideAttrs {
                postPatch = ''
                  rm -rf substrate/frame/contracts/fixtures/build || true
                  rm -rf substrate/frame/contracts/fixtures/contracts/common || true
                  rm -rf substrate/primitives/state-machine/fuzz || true
                '';
              }
            else
              drv;
          };
        # Build the workspace dependencies separately
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          inherit cargoVendorDir;
        });

        partner-chains-demo-node = craneLib.buildPackage (commonArgs // {
          pname = "partner-chains-demo-node";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          inherit cargoArtifacts;

          # Git commit hash for partner-chains CLI --version flag
          SUBSTRATE_CLI_GIT_COMMIT_HASH = "dev"; # self.dirtyShortRev or self.shortRev;
        });

        cargoTest = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });

        cargoClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
        });

        cargoFmt = craneLib.cargoFmt {
          inherit (commonArgs) pname src;
        };

        devShell = craneLib.devShell ({
          name = "partner-chains-demo-node-shell";
          # Inherit inputs from other build artifacts
          inputsFrom = [ partner-chains-demo-node ];

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

      in { # Main flake outputs section
        checks = {
          # Build the crate as part of `nix flake check'
          inherit partner-chains-demo-node cargoTest cargoFmt devShell;
        };

        packages = {
          inherit partner-chains-demo-node;
          default = partner-chains-demo-node;
          oci-image = n2c.packages.${system}.nix2container.buildImage {
            name = "partner-chains-demo-node";
            config = {
              Entrypoint = [ "${partner-chains-demo-node}/bin/partner-chains-demo-node" ];
              Expose = [
                "30333/tcp"
                "9615/tcp"
                "9933/tcp"
                "9944/tcp"
              ];
              Volumes = { "/data" = {}; };
            };
          };
        };

        devShells.default = devShell;

        formatter = pkgs.nixfmt-rfc-style;
      });

  nixConfig = {
    allow-import-from-derivation = true;
    accept-flake-config = true;
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://cache.iog.io"
      "https://ci.sc.iog.io/partner-chains"
    ];
    extra-trusted-public-keys = [
      "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "partner-chains:j9StpxUY/znqFqaevhQRxCH4Hi0F4rCGXDiUSjz+kew="
    ];
  };
}
