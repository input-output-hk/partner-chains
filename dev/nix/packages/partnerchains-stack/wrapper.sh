#!/bin/sh

DIR="$(dirname "$0")"

# Make sure we're running in the root project dir
cd "$(git rev-parse --show-toplevel)" || exit 1

NAMESPACE_FLAGS=""
OTHER_ARGS=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    -n|--namespace)
      # Check if the next argument exists and is not another flag
      if [ -n "$2" ] && [ "${2#--}" = "$2" ]; then
        NAMESPACE_FLAGS="$NAMESPACE_FLAGS --namespace $2"
        shift 2
      else
        echo "Error: $1 requires a non-empty argument." >&2
        exit 1
      fi
      ;;
    *)
      OTHER_ARGS="$OTHER_ARGS $1"
      shift
      ;;
  esac
done

if [ -z "$NAMESPACE_FLAGS" ]; then
  NAMESPACE_FLAGS="--namespace preview"
fi

set -- $NAMESPACE_FLAGS $OTHER_ARGS

exec "$DIR/partnerchains-stack-unwrapped" "$@"
