{ fetchzip, stdenv, ... }:

let
  version = "2.10.0";
  kupoLinux = fetchzip {
    url = "https://github.com/CardanoSolutions/kupo/releases/download/v2.10/kupo-v${version}-x86_64-linux.zip";
    hash = "sha256:UG2On7TATvDnPgeD6IgEyvF+WwRbwjq553dEG63Z8+M=";
    stripRoot = false;
    version = "${version}";
    name = "kupo-${version}";
    postFetch = "chmod +x $out/bin/kupo";
  };

  kupoDarwin = fetchzip {
    url = "https://github.com/CardanoSolutions/kupo/releases/download/v2.10/kupo-v${version}-aarch64-macos.zip";
    hash = "sha256:7WJWgLWz5xe3oO9rpFpcGMBUXa5fpqXjp5KaKY3EWk0=";
    stripRoot = false;
    version = "${version}";
    name = "kupo-${version}";
    postFetch = "chmod +x $out/bin/kupo";
  };

in
if stdenv.isDarwin then kupoDarwin else kupoLinux
