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
    trustlessPkgs = inputs'.trustless-sidechain.packages;
  in {
    packages = {
      inherit (trustlessPkgs) sidechain-main-cli-image;
    };
  };
}
