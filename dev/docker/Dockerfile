FROM partner-chains-node-builder:latest AS builder

FROM docker.io/library/ubuntu:25.10

COPY --from=builder /partner-chains-node/docker-build/target/release/partner-chains-demo-node /usr/local/bin/partner-chains-node

USER root

RUN useradd -m -u 1001 -U -s /bin/sh -d /substrate substrate && \
	mkdir -p /data /substrate/.local/share/partner-chains-node && \
	chown -R substrate:substrate /data && \
	ln -s /data /substrate/.local/share/partner-chains-node && \
	# Sanity checks
	ldd /usr/local/bin/partner-chains-node && \
	# Unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
	# Check if executable works in this container
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
