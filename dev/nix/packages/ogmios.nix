{ fetchzip, stdenv, ... }:
let
  version = "6.11.0";
  ogmiosLinux = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-x86_64-linux.zip";
    hash = "sha256-xwWEQn0yNJjAFm/MRBXXaLs6gpscYn4XRUUQwl/+Wnc=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

  ogmiosDarwin = fetchzip {
    url = "https://github.com/CardanoSolutions/ogmios/releases/download/v${version}/ogmios-v${version}-aarch64-macos.zip";
    hash = "sha256-fZyFClgXyCVe2bvrKX3JKsllwtpbAkL8wMylJVTv85E=";
    stripRoot = false;
    version = "${version}";
    name = "ogmios-${version}";
    postFetch = "chmod +x $out/bin/ogmios";
  };

in
if stdenv.isLinux then ogmiosLinux else ogmiosDarwin
