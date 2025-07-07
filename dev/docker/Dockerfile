FROM partner-chains-node-builder:latest AS builder
FROM partner-chains-node-runner:latest

COPY --from=builder /partner-chains-node/partner-chains-demo-node /usr/local/bin/partner-chains-node

RUN /usr/local/bin/partner-chains-node --version
RUN chown substrate:substrate /usr/local/bin/partner-chains-node && chmod +x /usr/local/bin/partner-chains-node

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
