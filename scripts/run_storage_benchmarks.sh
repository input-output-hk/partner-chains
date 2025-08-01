while getopts 'bfp:v' flag; do
  case "${flag}" in
    b)
      skip_build='true'
      ;;
    *)
      echo "Bad options. Check Script."
      exit 1
      ;;
  esac
done


if [ "$skip_build" != true ]
then
  echo "[+] Compiling Substrate benchmarks..."
  cargo build --profile=production --locked --features=runtime-benchmarks --bin partner-chains-node
fi

SIDECHAINS_NODE=./target/production/partner-chains-node

ERR_FILE="benchmarking_errors.txt"
rm -f $ERR_FILE

export CARDANO_DATA_SOURCE="mock"

echo "[+] Benchmarking storages"
OUTPUT=$(
  $SIDECHAINS_NODE benchmark storage \
  --state-version=1 \
  --warmups=1 \
  --weight-path="./runtime/src/weights/" 2>&1
)
if [ $? -ne 0 ]; then
  echo "$OUTPUT" >> "$ERR_FILE"
  echo "[-] Failed to benchmark storages. Error written to $ERR_FILE; continuing..."
fi


# NOTE:   
# Removing the below --base-path argument from the above benchmark command means that the storage benchmarks will be taken from genesis block. 
# The benchmarking ideally should be performed against a substrate data volume from a live chain, or a snapshot to avoid corrupting this volume
# Alas this sounds very complicated to handle in CI, and we don't yet see a requirement for this so are leaving this for now...
  
#  --base-path="/tmp/alice" \
