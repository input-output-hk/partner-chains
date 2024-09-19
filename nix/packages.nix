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

    # These need to be specified because cardano.nix provides multiples
    kupoVersion = "2.9.0";
    ogmiosVersion = "6.5.0";

    flake-compat = import inputs.flake-compat;
    cardanoPackages = (flake-compat { src = inputs.cardano-node; }).defaultNix.packages.${system};
    dbSyncPackages = (flake-compat { src = inputs.cardano-dbsync; }).defaultNix.packages.${system};
    smartContractsPkgs = (flake-compat { src = inputs.smart-contracts; }).defaultNix.packages.${system};
    cardanoExtraPkgs = (flake-compat { src = inputs.cardano-nix; }).defaultNix.packages.${system};

  in {
    packages = {
      inherit (smartContractsPkgs) pc-contracts-cli;
      inherit (cardanoPackages) cardano-node cardano-cli cardano-testnet;
      inherit (dbSyncPackages) "cardano-db-sync:exe:cardano-db-sync";
      kupo = cardanoExtraPkgs."kupo-${kupoVersion}";
      ogmios = cardanoExtraPkgs."ogmios-${ogmiosVersion}";
      partnerchains-stack = pkgs.stdenv.mkDerivation {
        name = "partnerchains-stack";
        phases = [ "installPhase" ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        installPhase = ''
          mkdir -p $out/bin
          cp ${self'.packages.partnerchains-stack-unwrapped}/bin/partnerchains-stack-unwrapped \
            $out/bin/partnerchains-stack
          wrapProgram $out/bin/partnerchains-stack \
            --run "cd \$(${pkgs.git}/bin/git rev-parse --show-toplevel) || exit 1"
        '';
      };
    };
  };
}
