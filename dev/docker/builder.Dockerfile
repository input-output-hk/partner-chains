FROM --platform=linux/amd64 docker.io/paritytech/ci-unified:bullseye-1.81.0-2024-11-19-v202411281558 AS builder

WORKDIR /partner-chains-node
COPY . /partner-chains-node

# Set environment variables to reduce memory usage and symlink issues
ENV CARGO_BUILD_JOBS=1
ENV RUSTFLAGS="-C target-cpu=native"
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN mkdir -p docker-build/cargo-home || echo "cargo-home already exists" && \
	export CARGO_HOME="$(pwd)/docker-build/cargo-home"
RUN --mount=type=ssh cargo build --release --target-dir=docker-build/target
