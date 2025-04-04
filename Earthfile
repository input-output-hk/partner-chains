VERSION 0.8
ARG --global PROFILE=release
ARG --global FEATURES

ci-pre-merge:
  BUILD +build
  BUILD +test
  BUILD +licenses
  BUILD +fmt
  BUILD +clippy
  BUILD +chainspecs
  ARG image=partner-chains-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

ci-post-merge:
  BUILD +build
  BUILD +chainspecs
  ARG image=partner-chains-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

ci-workflow-dispatch:
  BUILD +build
  BUILD +test
  BUILD +licenses
  BUILD +fmt
  BUILD +chainspecs
  ARG image=partner-chains-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

setup:
  FROM ubuntu:24.04
  WORKDIR /build
  ENV CARGO_HOME=/root/.cargo

  CACHE /var/lib/apt/lists
  CACHE /var/cache/apt/archives
  RUN apt-get update && apt-get install -y \
      build-essential \
      curl \
      git \
      python3 \
      python3-pip \
      python3-venv \
      protobuf-compiler \
      clang \
      cmake \
      libssl-dev \
      pkg-config \
      jq \
      libjq-dev \
      unzip

  # Install recent protoc
  ENV PB_REL="https://github.com/protocolbuffers/protobuf/releases"
  RUN curl -LO $PB_REL/download/v29.3/protoc-29.3-linux-x86_64.zip
  RUN unzip protoc-29.3-linux-x86_64.zip -d /usr/local
  RUN protoc --version

  ENV PIP_CACHE_DIR=/root/.cache/pip
  CACHE /root/.cache/pip
  RUN pip3 install --break-system-packages tomlq toml

  # Install rustup
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  ENV PATH="/root/.cargo/bin:${PATH}"

  # copy pre-existing $CARGO_HOME artifacts into the cache
  #RUN cp -rl $CARGO_HOME /tmp/cargo
  #CACHE --sharing shared --id cargo $CARGO_HOME
  #RUN cp -rua /tmp/cargo/. $CARGO_HOME && rm -rf /tmp/cargo
  COPY Cargo.* .rustfmt.toml rust-toolchain.toml .

  # Install the toolchain
  RUN rustup toolchain install
  RUN rustup show
  RUN cargo install --locked --version 0.1.68 cargo-chef && cp "$CARGO_HOME/bin/cargo-chef" /usr/local/bin

  # Add Linux target
  RUN rustup target add x86_64-unknown-linux-gnu

source:
  FROM +setup
  ARG CRATES=$(tomlq -r .workspace.members[] Cargo.toml)
  COPY .git .git
  COPY .gitignore .gitignore
  FOR crate IN $CRATES
      COPY --dir $crate $crate
  END

build:
  FROM +source
  #CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo build --locked --profile=$PROFILE --features=$FEATURES
  RUN ./target/*/partner-chains-demo-node --version
  SAVE ARTIFACT target/*/partner-chains-demo-node AS LOCAL partner-chains-node
  SAVE ARTIFACT target/*/partner-chains-demo-node AS LOCAL partner-chains-node-artifact

test:
  FROM +build
  DO github.com/earthly/lib:3.0.2+INSTALL_DIND
  #CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo test --no-run --locked --profile=$PROFILE --features=$FEATURES,runtime-benchmarks
  WITH DOCKER
    RUN cargo test --locked --profile=$PROFILE --features=$FEATURES,runtime-benchmarks
  END

licenses:
    FROM +source
    COPY scripts/validate_workspace_licenses.py validate_workspace_licenses.py
    RUN pip3 install --break-system-packages toml
    RUN cargo install --locked cargo-license
    RUN python3 validate_workspace_licenses.py

fmt:
  FROM +source
  #CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo fmt --check

clippy:
  FROM +source
  #CACHE --sharing shared --id cargo $CARGO_HOME
  ENV RUSTFLAGS="-Dwarnings"
  RUN cargo clippy --all-targets --all-features

docker:
    FROM ubuntu:24.04
    ARG image=partner-chains-node
    ARG tags

    RUN apt-get update && apt-get install -y \
        ca-certificates \
        libgcc-s1 \
        libstdc++6 \
        libc6 \
        libssl3 \
        zlib1g \
        libgomp1 \
        curl \
        wget \
        vim \
        dnsutils \
        jq \
        htop \
        && rm -rf /var/lib/apt/lists/*

    RUN useradd -m -u 1010 -U -s /bin/sh -d /substrate substrate \
        && mkdir -p /data /substrate/.local/share/partner-chains-node \
        && chown -R substrate:substrate /data /substrate \
        && ln -s /data /substrate/.local/share/partner-chains-node

    COPY +build/partner-chains-demo-node /usr/local/bin/partner-chains-node
    RUN /usr/local/bin/partner-chains-node --version
    RUN chown substrate:substrate /usr/local/bin/partner-chains-node && chmod +x /usr/local/bin/partner-chains-node

    USER substrate

    EXPOSE 30333
    EXPOSE 9615
    EXPOSE 9933
    EXPOSE 9944

    VOLUME ["/data"]

    ENTRYPOINT ["/usr/local/bin/partner-chains-node"]

		FOR tag IN $tags
		    SAVE IMAGE --push $image:$tag
		END

INSTALL:
  FUNCTION
  COPY +build/partner-chains-demo-node /usr/local/bin/partner-chains-node

  RUN ldd /usr/local/bin/partner-chains-node \
      && /usr/local/bin/partner-chains-node --version

chainspecs:
  FROM +setup
  DO +INSTALL

  RUN blahblah
  # Devnet
  COPY dev/envs/devnet/.envrc dev/envs/devnet/.envrc
  COPY dev/envs/devnet/addresses.json dev/envs/devnet/addresses.json
  RUN . ./dev/envs/devnet/.envrc \
      && partner-chains-node build-spec --chain local --disable-default-bootnode > devnet_chain_spec.json
  SAVE ARTIFACT devnet_chain_spec.json AS LOCAL devnet_chain_spec.json

  # ci-preview
  COPY dev/envs/ci-preview/.envrc dev/envs/ci-preview/.envrc
  COPY dev/envs/ci-preview/addresses.json dev/envs/ci-preview/addresses.json
  RUN . ./dev/envs/ci-preview/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode > ci_preview_chain_spec.json
  SAVE ARTIFACT ci_preview_chain_spec.json AS LOCAL ci_preview_chain_spec.json

  # staging-preview
  COPY dev/envs/staging-preview/.envrc dev/envs/staging-preview/.envrc
  COPY dev/envs/staging-preview/addresses.json dev/envs/staging-preview/addresses.json
  RUN . ./dev/envs/staging-preview/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode > staging_preview_chain_spec.json
  SAVE ARTIFACT staging_preview_chain_spec.json AS LOCAL staging_preview_chain_spec.json

  # staging-preprod
  COPY dev/envs/staging-preprod/.envrc dev/envs/staging-preprod/.envrc
  COPY dev/envs/staging-preprod/addresses.json dev/envs/staging-preprod/addresses.json
  RUN . ./dev/envs/staging-preprod/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode > staging_preprod_chain_spec.json
  SAVE ARTIFACT staging_preprod_chain_spec.json AS LOCAL staging_preprod_chain_spec.json
