{ self, inputs, ... }: {
  perSystem = { inputs', self', pkgs, system, ... }:
    let
      isLinux = pkgs.lib.hasSuffix "linux" system;
      isDarwin = pkgs.lib.hasSuffix "darwin" system;
      fenixPkgs = inputs'.fenix.packages;
      rustToolchain = fenixPkgs.fromToolchainFile {
          file = ../../rust-toolchain.toml;
          sha256 = "X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
        };
    in
    {
      devShells = {

        # This shell is provided only for generating cargo docs which requires
        # cargo doc from nightly to generate an index page
        nightly = pkgs.mkShell rec {
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ rustToolchain pkgs.stdenv.cc.cc pkgs.libz ];

          nightlyCargo = (fenixPkgs.toolchainOf {
            channel = "nightly";
            sha256 = "sha256-Xq3Xj6F7Rsi0vbSVY+HO5YdhxfEKgGhJ9259iDozjDs=";
          }).cargo;

          gen-cargo-docs = pkgs.writeScriptBin "gen-cargo-docs" ''
            RUSTDOCFLAGS="--enable-index-page -Zunstable-options" SKIP_WASM_BUILD=1 ${nightlyCargo}/bin/cargo doc --no-deps
          '';

          packages = with pkgs; [
            gen-cargo-docs
            pkg-config
            protobuf
          ];
        };

        default = pkgs.mkShell {
          # envs needed for rust toochain
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ rustToolchain pkgs.stdenv.cc.cc pkgs.libz ];
          # https://github.com/NixOS/nixpkgs/issues/370494#issuecomment-2625163369
          CFLAGS =
            if isLinux then
              "-DJEMALLOC_STRERROR_R_RETURNS_CHAR_WITH_GNU_SOURCE"
            else
              "";

          # envs needed in order to construct some of the rust crates
          ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
          OPENSSL_NO_VENDOR = 1;
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          # Force skip support check of c++17 in CC crate
          CRATE_CC_NO_DEFAULTS = "1";

          packages = with pkgs;
            [
              bashInteractive

              # core tooling to share across linux/macos
              coreutils
              pkg-config
              protobuf
              libiconv
              openssl
              gnumake

              # tools for e2e testing
              docker-compose
              python312
              python312Packages.pip
              python312Packages.virtualenv
              sops

              # local development tools
              rustToolchain
              gawk
              cargo-edit
              cargo-license
              nixfmt-rfc-style

              # infra packages
              earthly
              awscli2
              kubectl

              # our local packages
              self'.packages.cardano-cli
            ] ++ (if isDarwin then
              [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ]
            else
              [ pkgs.clang ]);
        };
        process-compose = pkgs.mkShell {
          inputsFrom = [ self'.devShells.default ];
          packages = [ self'.packages.partnerchains-stack ];
          shellHook = ''
            echo "Partner Chains dependency stack devshell";
            echo "useage: -n <network> to specify networks."
          '';
        };
      };
    };
}
