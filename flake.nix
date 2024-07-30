{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Rust toolchains in nix
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Nix helpers
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    process-compose.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:tgunnoe/services-flake";

    # Sidechains deps
    trustless-sidechain.url = "github:input-output-hk/trustless-sidechain/v6.0.0-rc3";
    cardano-node.url = "github:IntersectMBO/cardano-node/1.35.7";
    cardano-dbsync.url = "github:IntersectMBO/cardano-db-sync/13.1.1.3";
    ogmios.url = "github:mlabs-haskell/ogmios";
    kupo.url = "github:mlabs-haskell/kupo-nixos";
  };
  outputs = inputs @ {
    self,
    nixpkgs,
    flake-parts,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "x86_64-darwin" "aarch64-darwin"];
      imports = [
        inputs.devshell.flakeModule
        inputs.process-compose.flakeModule
        ./nix/shell.nix
        ./nix/packages.nix
        ./nix/processes.nix
      ];
      flake.lib = import ./nix/lib.nix {inherit (nixpkgs) lib;};
    };
  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://cache.iog.io"
      "https://cache.sc.iog.io"
    ];
    extra-trusted-public-keys = [
      "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "cache.sc.iog.io:b4YIcBabCEVKrLQgGW8Fylz4W8IvvfzRc+hy0idqrWU="
    ];
  };
}
