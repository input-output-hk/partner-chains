FROM paritytech/ci-unified:bullseye-1.77.0-2024-04-10-v202406031250

RUN apt-get update && apt-get install -y git protobuf-compiler jq &&  \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY ./entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
