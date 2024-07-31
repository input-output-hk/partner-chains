FROM docker.io/paritytech/ci-unified:bullseye-1.77.0-2024-04-10-v202406031250 as builder

WORKDIR /partner-chains-node
COPY . /partner-chains-node

RUN mkdir -p docker-build/cargo-home || echo "cargo-home already exists" && \
    export CARGO_HOME="$(pwd)/docker-build/cargo-home"
RUN --mount=type=ssh cargo build --release --target-dir=docker-build/target
