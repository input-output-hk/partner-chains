#!/bin/bash

chmod +x /docker-entrypoint-initdb.d/init.sh
exec docker-entrypoint.sh "$@"
