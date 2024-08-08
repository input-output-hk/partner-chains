{
  self,
  inputs,
  ...
}: {
  perSystem = {
    inputs',
    lib,
    pkgs,
    system,
    ...
  }: let
    smartContractsPkgs = inputs'.partner-chains-smart-contracts.packages;
  in {
    packages = {
      inherit (smartContractsPkgs) sidechain-main-cli-image;
    };
  };
}
