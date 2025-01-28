{
  self,
  inputs,
  ...
}: {
  perSystem = {
    inputs',
    self',
    pkgs,
    system,
    ...
  }: let
    isDarwin = pkgs.lib.hasSuffix "darwin" system;
    fenixPkgs = inputs'.fenix.packages;
    rustToolchain = with fenixPkgs;
      fromToolchainFile {
        file = ../../rust-toolchain.toml;
        sha256 = "VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
      };
  in {
    devShells = {
      default = pkgs.mkShell {
        # envs needed for rust toochain
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        LD_LIBRARY_PATH = "${rustToolchain}/lib";

        # envs needed in order to construct some of the rust crates
        ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
        OPENSSL_NO_VENDOR = 1;
        OPENSSL_DIR = "${pkgs.openssl.dev}";
        OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
        packages = with pkgs; [
          # core tooling to share across linux/macos
          coreutils
          pkg-config
          protobuf
          libiconv
          openssl
          gnumake

          # tools for e2e testing
          docker-compose
          python311
          python311Packages.pip
          python311Packages.virtualenv
          sops

          # local development tools
          rustToolchain
          nodejs
          gawk
          cargo-edit
          cargo-license
          perl

          # infra packages
          earthly
          awscli2
          kubectl
          kubernetes-helm

          # our local packages
          self'.packages.cardano-cli
          self'.packages.pc-contracts-cli
        ]
        ++ (
          if isDarwin
          then
            [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ]
          else [ pkgs.clang ]
        );
      };
      process-compose = pkgs.mkShell {
        inputsFrom = [ self'.devShells.default ];
        packages = [
          self'.packages.partnerchains-stack
        ];
        shellHook = ''
          echo "Partner Chains dependency stack devshell";
          echo "useage: -n <network> to specify networks."
        '';
      };
    };
  };
}
