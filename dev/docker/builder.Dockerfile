FROM docker.io/paritytech/ci-unified:latest AS builder

WORKDIR /partner-chains-node
COPY . /partner-chains-node

ENV RUSTUP_HOME="/partner-chains-node/docker-build/rustup-home"
ENV CARGO_HOME="/partner-chains-node/docker-build/cargo-home"
RUN --mount=type=cache,target=/partner-chains-node/docker-build \
	cargo build --release --target-dir=docker-build/target && \
	cp docker-build/target/release/partner-chains-demo-node /partner-chains-node/partner-chains-demo-node
