FROM cardanosolutions/cardano-node-ogmios:v6.11.0_10.1.4

COPY ./config /config
COPY ./busybox /busybox
COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
