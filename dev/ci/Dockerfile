FROM ubuntu:24.04

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
    ncat \
    expect \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1010 -U -s /bin/sh -d /substrate substrate \
    && mkdir -p /data /substrate/.local/share/partner-chains-node \
    && chown -R substrate:substrate /data /substrate \
    && ln -s /data /substrate/.local/share/partner-chains-node

COPY partner-chains-demo-node /usr/local/bin/partner-chains-node
RUN /usr/local/bin/partner-chains-node --version
RUN chown substrate:substrate /usr/local/bin/partner-chains-node && chmod +x /usr/local/bin/partner-chains-node

USER substrate

EXPOSE 30333
EXPOSE 9615
EXPOSE 9933
EXPOSE 9944

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/partner-chains-node"]
