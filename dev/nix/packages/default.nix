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
    rustToolchain = let
      fenixPkgs = inputs'.fenix.packages;
      rustToolchain = with fenixPkgs;
        fromToolchainFile {
          file = ../../../rust-toolchain.toml;
          sha256 = "VZZnlyP69+Y3crrLHQyJirqlHrTtGTsyiSnZB8jEvVo=";
        };
    in rustToolchain;
    craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;



  in {
    packages = rec {
      inherit rustToolchain;
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";


      commonArgs = {
        src = craneLib.cleanCargoSource ../../../.;
        strictDeps = true;

        buildInputs = with pkgs; [
          # Add additional build inputs here
          coreutils
          pkg-config
          protobuf
          libiconv
          openssl
          gnumake
        ] ++ lib.optionals pkgs.stdenv.isDarwin [
          # Additional darwin specific inputs can be set here
          pkgs.libiconv
        ];
      };


      individualCrateArgs = commonArgs // {
        #inherit cargoArtifacts;
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        inherit (craneLib.crateNameFromCargoToml {
          src = craneLib.cleanCargoSource ../../../.; }) version;
        # NB: we disable tests since we'll run them all via cargo-nextest
        doCheck = false;
      };

      fileSetForCrate = crate: lib.fileset.toSource {
        root = ../../../.;
        fileset = lib.fileset.unions [
          ../../../Cargo.toml
          ../../../Cargo.lock
          #(craneLib.fileset.commonCargoSources ./crates/my-common)
          #(craneLib.fileset.commonCargoSources ./crates/my-workspace-hack)
          (craneLib.fileset.commonCargoSources crate)
        ];
      };

      # Derivation for partner-chains-node
      partner-chains-node = craneLib.buildPackage {
        src = fileSetForCrate ../../../node/node;
        strictDeps = true;
        cargoExtraArgs = "--release --bin partner-chains-node";

        doCheck = false;

      };

      # Derivation for partner-chains-cli
      # partner-chains-cli = craneLib.buildPackage {
      #   pname = "partner-chains-cli";
      #   version = "0.1.0";
      #   src = src;
      #   cargoExtraArgs = "--release --bin partner-chains-cli";
      #   nativeBuildInputs = buildInputs;
      #   doCheck = false;
      # };

      ogmios = pkgs.callPackage ./ogmios.nix { };
      process-compose = pkgs.process-compose.overrideAttrs (oldAttrs: {
        patches = [ ./pc.patch ];
      });
      partnerchains-stack = pkgs.callPackage ./partnerchains-stack { inherit (self'.packages) partnerchains-stack-unwrapped; };
    };
  };
}
