VERSION 0.8
ARG --global PROFILE=release
ARG --global FEATURES

ci:
  BUILD +build
  BUILD +test
  BUILD +fmt
  BUILD +chainspecs
  ARG image=sidechains-substrate-node
  ARG tags
  BUILD +docker --image=$image --tags=$tags

setup:
  FROM paritytech/ci-unified:bullseye-1.81.0-2024-09-11-v202409111034
  WORKDIR /build

  # copy pre-existing $CARGO_HOME artifacts into the cache
  RUN cp -rl $CARGO_HOME /tmp/cargo
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cp -rua /tmp/cargo/. $CARGO_HOME && rm -rf /tmp/cargo

  COPY Cargo.* .rustfmt.toml rust-toolchain.toml .
  RUN rustup show
  RUN cargo install --locked cargo-chef && cp "$CARGO_HOME/bin/cargo-chef" /usr/local/bin

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
  CACHE --sharing shared --id cargo $CARGO_HOME
  ARG EARTHLY_GIT_HASH
  RUN cargo build --locked --profile=$PROFILE --features=$FEATURES
  SAVE ARTIFACT target/*/partner-chains-node AS LOCAL partner-chains-node
  SAVE ARTIFACT target/*/partner-chains-node AS LOCAL partner-chains-node-artifact
  SAVE ARTIFACT target/*/partner-chains-cli AS LOCAL partner-chains-cli-artifact

test:
  FROM +build
  LET WASM_BUILD_STD=0
  DO github.com/earthly/lib:3.0.2+INSTALL_DIND
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN cargo test --no-run --locked --profile=$PROFILE --features=$FEATURES,runtime-benchmarks
  WITH DOCKER
    RUN cargo test --locked --profile=$PROFILE --features=$FEATURES,runtime-benchmarks
  END

fmt:
  FROM +source
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN find runtime/src/weights -type f -name '*.rs' -exec cargo fmt -- {} +
  RUN cargo fmt --check

docker:
    FROM debian:bullseye-slim
    ARG image=sidechains-substrate-node
    ARG tags

    DO +INSTALL

    RUN useradd -m -u 1000 -U -s /bin/sh -d /substrate substrate \
        && mkdir -p /data /substrate/.local/share/partner-chains-node \
        && chown -R substrate:substrate /data /substrate \
        # remove package managers
        && rm -rf /usr/bin/apt* /usr/bin/dpkg* \
        && ln -s /data /substrate/.local/share/partner-chains-node

    USER substrate

    #p2p
    EXPOSE 30333
    #prometheus exporter
    EXPOSE 9615
    #JSON-RPC HTTP
    EXPOSE 9933
    #JSON-RPC WS
    EXPOSE 9944

    VOLUME ["/data"]

    ENTRYPOINT ["./usr/local/bin/partner-chains-node"]

    ARG EARTHLY_GIT_HASH
    ENV EARTHLY_GIT_HASH=$EARTHLY_GIT_HASH

    FOR tag IN $EARTHLY_GIT_HASH $tags
        SAVE IMAGE --push $image:$tag
    END

deps:
    FROM +source
    COPY +build/partner-chains-node .
    # calculate libary deps
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
      && touch node/src/lib.rs

fetch-deps:
  FROM +mock
  CACHE --sharing shared --id cargo $CARGO_HOME
  RUN --ssh cargo fetch --locked

INSTALL:
  FUNCTION
  COPY +build/partner-chains-node /usr/local/bin
  COPY +deps/deps /tmp/deps.tgz

  # install deps
  RUN tar -v -C / -xzf /tmp/deps.tgz \
      && rm -rf /tmp/deps.tgz

  # Sanity checks
  RUN ldd /usr/local/bin/partner-chains-node \
      && /usr/local/bin/partner-chains-node --version

chainspecs:
  FROM +setup
  DO +INSTALL

  COPY envs/devnet/.envrc envs/devnet/.envrc
  COPY envs/devnet/addresses.json envs/devnet/addresses.json

  COPY envs/staging-preview/.envrc envs/staging-preview/.envrc
  COPY envs/staging-preview/addresses.json envs/staging-preview/addresses.json

  COPY envs/staging-preprod/.envrc envs/staging-preprod/.envrc
  COPY envs/staging-preprod/addresses.json envs/staging-preprod/addresses.json

  RUN . ./envs/devnet/.envrc \
      && partner-chains-node build-spec --chain local --disable-default-bootnode --raw > devnet_chain_spec.json
  RUN . ./envs/staging-preview/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode --raw > staging_preview_chain_spec.json
  RUN . ./envs/staging-preprod/.envrc \
      && partner-chains-node build-spec --chain staging --disable-default-bootnode --raw > staging_preprod_chain_spec.json

  SAVE ARTIFACT devnet_chain_spec.json AS LOCAL devnet_chain_spec.json
  SAVE ARTIFACT staging_preview_chain_spec.json AS LOCAL staging_preview_chain_spec.json
  SAVE ARTIFACT staging_preprod_chain_spec.json AS LOCAL staging_preprod_chain_spec.json
