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
    # Read the shell.toml file
    shellConfig = builtins.fromTOML (builtins.readFile ../shell.toml);

    isDarwin = pkgs.lib.hasSuffix "darwin" system;
    fenixPkgs = inputs'.fenix.packages;
    # Define the rust toolchain
    rustToolchain = fenixPkgs.fromToolchainFile {
      file = ../../rust-toolchain.toml;
      sha256 = "VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
    };

    # Function to resolve package names from shellConfig.packages.list
    resolvePackage = pkg:
      if pkgs ? pkg then pkgs.${pkg}
      else throw "Package '${pkg}' not found in nixpkgs";


    # Resolve the packages
    tracePackages =  builtins.trace "Package names: ${builtins.toString (shellConfig.packages.list)}" shellConfig.packages.list;

    packages = map resolvePackage shellConfig.packages;

  in {
    devShells = {
      default = pkgs.mkShell ({
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        LD_LIBRARY_PATH = "${rustToolchain}/lib";
        ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib/";
        OPENSSL_NO_VENDOR = 1;
        OPENSSL_DIR  = "${pkgs.openssl.dev}";
        OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

        # Required packages for shell functionality
        packages = [
          (if isDarwin
           then
             pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
           else pkgs.clang
          )
          rustToolchain
          pkgs.coreutils
          pkgs.pkg-config
          pkgs.protobuf
          pkgs.libiconv
          pkgs.openssl
          pkgs.just
        ] ++ packages;

        # Additional shell hook
        shellHook = ''
          echo 'Welcome to Partner Chains'
          echo 'run just -l to see a list of actions'
      '';
      } // shellConfig.envs.default);
      devnet = pkgs.mkShell ({
        inputsFrom = [ self'.devShells.default ];
      } // shellConfig.envs.devnet);
    };
  };
}
