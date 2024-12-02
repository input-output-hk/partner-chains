{ fetchzip, stdenv, ... }:
let
  version = "6.9.0";
  ogmiosLinux = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-x86_64-linux.zip";
    hash = "sha256-i6J3ybpxdZvd/e7ktIW4Dvf7Crobs60jwEYHfFxnsJw=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

  ogmiosDarwin = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-aarch64-macos.zip";
    hash = "sha256-mlrL7D1muYLmcAguUotgFRKJrvKXzPTrEj8uzINkt8s=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

in
if stdenv.isLinux then ogmiosLinux else ogmiosDarwin
