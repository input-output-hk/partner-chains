{ fetchzip, stdenv, version, ... }:

fetchzip {
  url = "https://github.com/input-output-hk/partner-chains-smart-contracts/releases/download/v${version}/pc-contracts-cli-v${version}.zip";
  hash = "sha256-Sp94vyyjI1lfRar6TJX1YRD/eOYkbK/t7dplLZx7+iA=";
  stripRoot = false;
  version = "${version}";
  name = "pc-contracts-cli-${version}";
  postFetch = ''
      mkdir -p $out/bin
      mv $out/pc-contracts-cli $out/bin
      chmod +x $out/bin/pc-contracts-cli
  '';
}
