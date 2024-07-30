ARG trustless_sidechain_image
ARG relay_base_image
FROM $trustless_sidechain_image as trustless-sidechain
FROM $relay_base_image
COPY --from=trustless-sidechain /bin/sidechain-main-cli /bin/sidechain-main-cli
COPY --from=trustless-sidechain /nix /nix
COPY config.json /local/config.json
COPY relay.skey /secrets/relay.skey
COPY init.skey /secrets/init.skey
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
