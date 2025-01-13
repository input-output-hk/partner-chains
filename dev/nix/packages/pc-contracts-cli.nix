{ fetchzip, stdenv, ... }:
let
  version = "7.0.2";
in fetchzip {
  url = "https://github.com/input-output-hk/partner-chains-smart-contracts/releases/download/v${version}/pc-contracts-cli-v${version}.zip";
  hash = "sha256-g2uUS+HD7+YjiczH16TNfg+Qy2Yhc+W1Rlf02pAR7B4=";
  stripRoot = false;
  version = "${version}";
  name = "pc-contracts-cli-${version}";
  postFetch = ''
      mkdir -p $out/bin
      mv $out/pc-contracts-cli $out/bin
      chmod +x $out/bin/pc-contracts-cli
  '';
}
