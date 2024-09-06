#! /bin/bash

chmod 600 /keys/*

echo "Waiting for Node 1 to prepare configuration files and set the target start time..."

while true; do
    if [ -f "/shared/cardano.start" ]; then
        target_time=$(cat /shared/cardano.start)
        echo "Node 1 is ready. Starting synchronization..."
        break
    else
        sleep 1
    fi
done

adjusted_target_time=$((target_time - 10))
current_epoch=$(date +%s%3N)
sleep_milliseconds=$((adjusted_target_time * 1000 - current_epoch))
sleep_seconds=$(($sleep_milliseconds / 1000))
remaining_milliseconds=$((sleep_milliseconds % 1000))
total_sleep_time=$(printf "%.3f" "$(echo "scale=3; $sleep_milliseconds / 1000" | /busybox bc)")
echo "Waiting for $total_sleep_time seconds until 10 seconds before the target time..."
sleep $total_sleep_time
echo "Current time is now: $(date +"%H:%M:%S.%3N"). Starting node..."

cardano-node run \
--topology /shared/node-3-topology.json \
--database-path /data/db \
--socket-path /data/node.socket \
--host-addr 0.0.0.0 \
--port 32010 \
--config /shared/node-3-config.json \
--shelley-kes-key /keys/kes.skey \
--shelley-vrf-key /keys/vrf.skey \
--shelley-operational-certificate /keys/node.cert &

touch /shared/cardano-node-3.ready

wait
