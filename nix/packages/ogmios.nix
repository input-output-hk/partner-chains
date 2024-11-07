{ fetchzip, stdenv, version, ... }:
let
  ogmiosLinux = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-x86_64-linux.zip";
    hash = "sha256-PM3tB6YdFsXRxGptDuxOvLke0m/08ySy4oV1WfIu//g=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

  ogmiosDarwin = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-aarch64-macos.zip";
    hash = "sha256-YcSUft/aH9o2F0o1CFcmrvSnSYs0RE1fPvFW6ihWVWM=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

in
if stdenv.isLinux then ogmiosLinux else ogmiosDarwin
