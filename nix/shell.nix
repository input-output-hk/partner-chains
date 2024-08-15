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
        file = ../rust-toolchain.toml;
        # Probably should be a flake input instead
        sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
      };
    packages = with pkgs;
      [
        coreutils
        protobuf
        rustToolchain # The rust toolchain we constructed above
        nodejs
        clang
        nodePackages.npm
		gnumake
      ]
      ++ (
        if isDarwin
        then
          with pkgs.darwin.apple_sdk_11_0; [
            frameworks.SystemConfiguration
            frameworks.CoreFoundation
            darwin.Libsystem
          ]
        else []
      );
    env = [
      {
        name = "RUST_SRC_PATH";
        value = "${rustToolchain}/lib/rustlib/src/rust/library";
      }
      {
        name = "LIBCLANG_PATH";
        value = "${pkgs.libclang.lib}/lib";
      }
      {
        name = "LD_LIBRARY_PATH";
        value = "${rustToolchain}/lib";
      }
      {
        name = "BINDGEN_EXTRA_CLANG_ARGS";
        value = with pkgs;
          if isDarwin
          then "-isystem ${darwin.Libsystem}/include"
          else "-isystem ${libclang.lib}/lib/clang/${lib.getVersion libclang}/include";
      }
      {
        name = "PATH";
        prefix = "${pkgs.coreutils}/bin";
      }
    ];
    # Main Categories which can include pkgs, or devShell-like sets
    # for commands and helpers
    commands = self.lib.categorize [
      {
        help = "Earthly, an easy to use CI tool";
        name = "earthly";
        package = pkgs.earthly;
        category = "CI/CD";
        pkgs = [
          pkgs.earthly
          pkgs.awscli2
          pkgs.kubectl
          pkgs.kubernetes-helm
        ];
      }
      {
        category = "Rust";
        pkgs = [
          {
            name = "check";
            help = "Check rustc and clippy warnings";
            command = ''
              set -x
              cargo check --all-targets
              cargo clippy --all-targets
            '';
          }
          {
            help = "Automatically fix rustc and clippy warnings";
            name = "fix";
            command = ''
              set -x
              cargo fix --all-targets --allow-dirty --allow-staged
              cargo clippy --all-targets --fix --allow-dirty --allow-staged
            '';
          }
        ];
      }
      {
        category = "Sidechains";
        pkgs = [
          {
            name = "cardano-cli";
            help = "CLI v1.35.7 that is used in sidechains dependency stack";
            # This command has some eval because of IFD
            command = "${inputs'.cardano-node.packages.cardano-cli}/bin/cardano-cli $@";
          }
        ];
      }
    ];
    extraCommands =
      commands
      ++ self.lib.categorize [
        {
          category = "Sidechains";
          pkgs = [
            {
              name = "sidechains-stack";
              help = "Run a process-compose stack of all of the dependencies";
              command = ''
                ${
                  if isDarwin
                  then self.packages.x86_64-darwin.sidechains-stack
                  else self'.packages.sidechains-stack
                }/bin/sidechains-stack $@
              '';
            }
            {
              name = "sidechain-main-cli";
              help = "CLI application to execute Trustless Sidechain Cardano endpoints";
              command = ''
                ${
                  if isDarwin
                  then inputs.trustless-sidechain.packages.x86_64-darwin.sidechain-main-cli
                  else inputs'.trustless-sidechain.packages.sidechain-main-cli
                }/bin/sidechain-main-cli $@
              '';
            }
          ];
        }
      ];
  in {
    devshells.default = {
      inherit packages env commands;
      name = "Sidechains Substrate Node Devshell";
    };
    devshells.process-compose = {
      inherit packages env;
      commands = extraCommands;
      name = "Sidechains Substrate Node Devshell with whole stack";
    };
    devshells.trustless-sidechain = {
      inherit packages env;
      commands = commands ++ [
        {
          category = "Sidechains";
          name = "sidechain-main-cli";
          help = "CLI application to execute Trustless Sidechain Cardano endpoints";
          command = ''
            ${
              if isDarwin
              then inputs.trustless-sidechain.packages.x86_64-darwin.sidechain-main-cli
              else inputs'.trustless-sidechain.packages.sidechain-main-cli
            }/bin/sidechain-main-cli $@
          '';
        }
      ];
      name = "Sidechains Substrate Node Devshell with Trustless CLI";
    };
  };
}
