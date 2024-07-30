#! /bin/sh
cd /local
while :
do
  echo "config.json:"
  cat config.json
  set +e
  /opt/docker/bin/relay --node-url "${NODE_URL}" --relay-signing-key-path /secrets/relay.skey --init-signing-key-path /secrets/init.skey --init-timeout "${INIT_TIMEOUT}"
  set -e
  sleep 60
done
echo "Done!"
