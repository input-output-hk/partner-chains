FROM paritytech/ci-unified:bullseye-1.81.0-2024-09-11-v202409111034

RUN apt-get update && apt-get install -y git protobuf-compiler jq &&  \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY ./entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
