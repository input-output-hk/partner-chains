FROM docker.io/library/ubuntu:25.04
COPY partner-chains-demo-node /usr/local/bin/partner-chains-node

RUN useradd -m -u 1001 -U -s /bin/sh -d /substrate substrate
RUN	mkdir -p /data /substrate/.local/share/partner-chains-node
RUN	chown -R substrate:substrate /data
RUN ln -s /data /substrate/.local/share/partner-chains-node
# Sanity checks
RUN ldd /usr/local/bin/partner-chains-node
RUN /usr/local/bin/partner-chains-node --version

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

ENTRYPOINT ["/usr/local/bin/partner-chains-node"]
