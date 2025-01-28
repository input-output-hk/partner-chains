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
  in {
    packages = {
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";
      kupo = pkgs.callPackage ./kupo.nix {  };
      ogmios = pkgs.callPackage ./ogmios.nix { };
      process-compose = pkgs.process-compose.overrideAttrs (oldAttrs: {
        patches = [ ./pc.patch ];
      });
      partnerchains-stack = pkgs.callPackage ./partnerchains-stack { inherit (self'.packages) partnerchains-stack-unwrapped; };

    };
  };
}
