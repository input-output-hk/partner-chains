VERSION 0.8
IMPORT ./dev/earthly/init
IMPORT ./dev/earthly/deps

ARG --global PROFILE=release
ARG --global FEATURES
ARG --global BIN=partner-chains-node

boot:
  FROM paritytech/ci-unified:bullseye-1.77.0-2024-04-10-v202406031250
  WORKDIR /build

  DO init+INSTALL

ci:
  BUILD +build
  BUILD +test
  BUILD +fmt
  BUILD +chainspecs

  ARG image=$BIN
  ARG tags
  BUILD +docker --image=$image --tags=$tags

docker:
    FROM debian:bullseye-slim
    ARG image=$BIN
    ARG tags

    DO +INSTALL

    RUN useradd -m -u 1000 -U -s /bin/sh -d /substrate substrate \
        && mkdir -p /data /substrate/.local/share/$BIN \
        && chown -R substrate:substrate /data /substrate \
        # remove package managers
        && rm -rf /usr/bin/apt* /usr/bin/dpkg* \
        && ln -s /data /substrate/.local/share/$BIN

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

    ENTRYPOINT ["./usr/local/bin/$BIN"]

    ARG EARTHLY_GIT_HASH
    ENV EARTHLY_GIT_HASH=$EARTHLY_GIT_HASH

    FOR tag IN $EARTHLY_GIT_HASH $tags
        SAVE IMAGE --push $image:$tag
    END

test:
  FROM +build
  LET WASM_BUILD_STD=0
  DO github.com/earthly/lib:3.0.3+INSTALL_DIND
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

build:
  FROM +source
  LET WASM_BUILD_STD=0
  CACHE --sharing shared --id cargo $CARGO_HOME
  ARG EARTHLY_GIT_HASH
  RUN cargo build --locked --profile=$PROFILE --features=$FEATURES -p $BIN
  SAVE ARTIFACT target/*/$BIN AS LOCAL $BIN

chainspecs:
  FROM +boot
  DO +INSTALL
  COPY dev/devnet/.envrc devnet/.envrc
  COPY dev/devnet/addresses.json devnet/addresses.json
  COPY dev/staging/.envrc staging/.envrc
  COPY dev/staging/addresses.json staging/addresses.json
  # `.` (dot) is equivalent of `source` in /bin/sh
  RUN . ./devnet/.envrc \
      && $BIN build-spec --chain local --disable-default-bootnode --raw > devnet_chain_spec.json
  RUN. ./staging/.envrc \
      && $BIN build-spec --chain staging --disable-default-bootnode --raw > staging_chain_spec.json
  SAVE ARTIFACT devnet_chain_spec.json AS LOCAL devnet_chain_spec.json
  SAVE ARTIFACT staging_chain_spec.json AS LOCAL staging_chain_spec.json

source:
  FROM +boot
  DO deps+COMPILE_SOURCE
  COPY --dir +build-deps/target .

build-deps:
  FROM +fetch-deps
  DO --pass-args deps+BUILD

fetch-deps:
  FROM +source-mock
  DO deps+FETCH

deps:
    FROM +source
    COPY +build/$BIN .
    DO deps+COMPILE_RUNTIME

source-mock:
  FROM +boot
  DO init+MOCK

INSTALL:
  FUNCTION
  COPY +build/$BIN /usr/local/bin
  COPY +deps/deps /tmp/deps.tgz
  DO --pass-args deps+INSTALL
