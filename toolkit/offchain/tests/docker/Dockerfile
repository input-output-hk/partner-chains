FROM cardanosolutions/cardano-node-ogmios:v6.9.0_10.1.2

COPY ./config /config
COPY ./busybox /busybox
COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
