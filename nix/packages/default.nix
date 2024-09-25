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
    kupoVersion = "2.9.0";
    ogmiosVersion = "6.6.2";

    flake-compat = import inputs.flake-compat;
    cardanoPackages = (flake-compat { src = inputs.cardano-node; }).defaultNix.packages.${system};
    dbSyncPackages = (flake-compat { src = inputs.cardano-dbsync; }).defaultNix.packages.${system};
    smartContractsPkgs = (flake-compat { src = inputs.smart-contracts; }).defaultNix.packages.${system};
    #cardanoExtraPkgs = (flake-compat { src = inputs.cardano-nix; }).defaultNix.packages.${system};
  in {
    packages = {
      inherit (smartContractsPkgs) pc-contracts-cli;
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";
      kupo = pkgs.callPackage ./kupo.nix { version = kupoVersion; };
      ogmios = pkgs.callPackage ./ogmios.nix { version = ogmiosVersion; };
      process-compose = pkgs.process-compose.overrideAttrs (oldAttrs: {
        patches = [ ./pc.patch ];
      });
      partnerchains-stack = pkgs.callPackage ./partnerchains-stack { inherit (self'.packages) partnerchains-stack-unwrapped; };

    };
  };
}
