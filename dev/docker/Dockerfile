FROM partner-chains-node-builder:latest AS builder
FROM partner-chains-node-runner:latest

COPY --from=builder /partner-chains-node/partner-chains-demo-node /usr/local/bin/partner-chains-node

RUN /usr/local/bin/partner-chains-node --version
RUN chown substrate:substrate /usr/local/bin/partner-chains-node && chmod +x /usr/local/bin/partner-chains-node

USER substrate

EXPOSE 30333
EXPOSE 9615
EXPOSE 9933
EXPOSE 9944

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/partner-chains-node"]
