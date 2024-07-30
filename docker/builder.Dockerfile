FROM docker.io/paritytech/ci-linux:1.70.0-bullseye as builder

WORKDIR /partner-chains-node
COPY . /partner-chains-node

RUN mkdir -p docker-build/cargo-home || echo "cargo-home already exists" && \
    export CARGO_HOME="$(pwd)/docker-build/cargo-home"
RUN --mount=type=ssh cargo test --release --target-dir=docker-build/target && \
	cargo build --release --target-dir=docker-build/target; \
