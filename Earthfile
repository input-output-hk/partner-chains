VERSION 0.8
ARG --global PROFILE=release
ARG --global FEATURES

ci-pre-merge:
  BUILD +build
  BUILD +test
  BUILD +licenses
  BUILD +fmt
  ARG image=partner-chains-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

ci-post-merge:
  BUILD +build
  BUILD +chainspecs
  ARG image=partner-chains-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

setup:
  FROM ubuntu:24.04
  WORKDIR /build
  ENV CARGO_HOME=/root/.cargo

  CACHE /var/lib/apt/lists
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
      && rm -rf /var/lib/apt/lists/*

  RUN pip3 install --break-system-packages tomlq toml

  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  ENV PATH="/root/.cargo/bin:${PATH}"

  # copy pre-existing $CARGO_HOME artifacts into the cache
  RUN cp -rl $CARGO_HOME /tmp/cargo
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cp -rua /tmp/cargo/. $CARGO_HOME && rm -rf /tmp/cargo
  COPY Cargo.* .rustfmt.toml rust-toolchain.toml .
  RUN rustup show
  RUN cargo install --locked --version 0.1.68 cargo-chef && cp "$CARGO_HOME/bin/cargo-chef" /usr/local/bin

  # Add Linux target
  RUN rustup target add x86_64-unknown-linux-gnu

source:
  FROM +setup
  ARG CRATES=$(tomlq -r .workspace.members[] Cargo.toml)
  FOR crate IN $CRATES
      COPY --dir $crate $crate
  END
  COPY --dir +build-deps/target .

build-deps:
  FROM +fetch-deps
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo --locked chef prepare
  RUN cargo --locked chef cook --profile=$PROFILE --features=$FEATURES
  SAVE ARTIFACT target

build:
  FROM +source
  LET WASM_BUILD_STD=0
  #ARG CACHE_KEY=$(find . -type f -name "*.rs" -o -name "*.toml" | sort | xargs cat | sha256sum)
  #CACHE --sharing shared --id cargo-build-$CACHE_KEY target
  CACHE --sharing shared --id cargo $CARGO_HOME
  ARG EARTHLY_GIT_HASH
  RUN cargo build --locked --profile=$PROFILE --features=$FEATURES
  #SAVE ARTIFACT target
  SAVE ARTIFACT target/*/partner-chains-node AS LOCAL partner-chains-node
  SAVE ARTIFACT target/*/partner-chains-node AS LOCAL partner-chains-node-artifact

test:
  FROM +build
  LET WASM_BUILD_STD=0
  DO github.com/earthly/lib:3.0.2+INSTALL_DIND
  CACHE --sharing shared --id cargo $CARGO_HOME
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
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo fmt --check

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
        && rm -rf /var/lib/apt/lists/*

    RUN useradd -m -u 1010 -U -s /bin/sh -d /substrate substrate \
        && mkdir -p /data /substrate/.local/share/partner-chains-node \
        && chown -R substrate:substrate /data /substrate \
        && ln -s /data /substrate/.local/share/partner-chains-node

    COPY +build/partner-chains-node /usr/local/bin/
    RUN chown substrate:substrate /usr/local/bin/partner-chains-node && chmod +x /usr/local/bin/partner-chains-node

    USER substrate

    EXPOSE 30333
    EXPOSE 9615
    EXPOSE 9933
    EXPOSE 9944

    VOLUME ["/data"]

    ENTRYPOINT ["/usr/local/bin/partner-chains-node"]

    ARG EARTHLY_GIT_HASH
    ENV EARTHLY_GIT_HASH=$EARTHLY_GIT_HASH

    FOR tag IN $EARTHLY_GIT_HASH $tags
        SAVE IMAGE --push $image:$tag
    END

deps:
    FROM +source
    COPY +build/partner-chains-node .
    RUN ldd partner-chains-node \
        | awk 'NF == 4 { system("echo " $3) }' \
        | tar -czf deps.tgz --files-from=-
    SAVE ARTIFACT deps.tgz deps

mock:
  FROM +setup
  ARG CRATES=$(tomlq -r '.workspace.members[]' Cargo.toml)
  ARG SRCS=$(tomlq -r '.workspace.members[] + "/src"' Cargo.toml)
  ARG LIBS=$(tomlq -r '.workspace.dependencies[]|select(type == "object" and has("path")).path + "/src/lib.rs"' Cargo.toml)
  FOR crate IN $CRATES
    COPY --if-exists $crate/Cargo.toml $crate/Cargo.toml
  END
  RUN mkdir -p $SRCS \
      && touch $LIBS \
      && for crate in $SRCS; do if [ ! -f $crate/lib.rs ]; then touch $crate/main.rs; fi; done \
      && touch node/node/src/lib.rs

fetch-deps:
  FROM +mock
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN --ssh cargo fetch --locked

INSTALL:
  FUNCTION
  COPY +build/partner-chains-node /usr/local/bin
  COPY +deps/deps /tmp/deps.tgz

  RUN tar -v -C / -xzf /tmp/deps.tgz \
      && rm -rf /tmp/deps.tgz

  RUN ldd /usr/local/bin/partner-chains-node \
      && /usr/local/bin/partner-chains-node --version

chainspecs:
  FROM +setup
  DO +INSTALL

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
  RUN . ./dev/envs/ci-preview/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode > staging_preview_chain_spec.json
  SAVE ARTIFACT staging_preview_chain_spec.json AS LOCAL staging_preview_chain_spec.json

  # staging-preprod
  COPY dev/envs/staging-preprod/.envrc dev/envs/staging-preprod/.envrc
  COPY dev/envs/staging-preprod/addresses.json dev/envs/staging-preprod/addresses.json
  RUN . ./dev/envs/staging-preprod/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode > staging_preprod_chain_spec.json
  SAVE ARTIFACT staging_preprod_chain_spec.json AS LOCAL staging_preprod_chain_spec.json
