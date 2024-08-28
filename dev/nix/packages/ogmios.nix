{ fetchzip, stdenv, version, ... }:
let
  ogmiosLinux = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-x86_64-linux.zip";
    hash = "sha256-KHOna+zDFJDVD20M2dTD71dcDKWMSXmqOPWMAYef9/4=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

  ogmiosDarwin = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-aarch64-macos.zip";
    hash = "sha256-eoL8aLwZlBd7R/1REYjN56Bk0t+NBNBTFg7KyGr78PE=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

in
if stdenv.isLinux then ogmiosLinux else ogmiosDarwin
