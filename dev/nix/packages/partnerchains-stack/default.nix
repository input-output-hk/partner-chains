{ partnerchains-stack-unwrapped, git, stdenv }:

stdenv.mkDerivation {
  pname = "partnerchains-stack";
  version = "1.0.0"; # Replace with your actual version

  # Specify dependencies
  nativeBuildInputs = [ git ];

  # Define the build phases
  phases = [ "installPhase" ];

  installPhase = ''
    mkdir -p $out/bin

    cp ${partnerchains-stack-unwrapped}/bin/partnerchains-stack-unwrapped \
      $out/bin/partnerchains-stack-unwrapped

    cp ${./wrapper.sh} $out/bin/partnerchains-stack

    chmod +x $out/bin/partnerchains-stack
  '';
}
