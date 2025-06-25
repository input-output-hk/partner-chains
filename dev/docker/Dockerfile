FROM --platform=linux/amd64 partner-chains-node-builder:latest AS builder

FROM --platform=linux/amd64 docker.io/library/ubuntu:20.04
COPY --from=builder /partner-chains-node/docker-build/target/release/partner-chains-demo-node /usr/local/bin/partner-chains-node

RUN useradd -m -u 1000 -U -s /bin/sh -d /substrate substrate && \
	mkdir -p /data /substrate/.local/share/partner-chains-node && \
	chown -R substrate:substrate /data && \
	ln -s /data /substrate/.local/share/partner-chains-node && \
	# Sanity checks
	ldd /usr/local/bin/partner-chains-node && \
	/usr/local/bin/partner-chains-node --version

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
