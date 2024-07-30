: "${RELAY_IMAGE_TAG:?Variable not set or empty.}"
export RELAY_IMAGE="relay:$RELAY_IMAGE_TAG"
echo "Using relay base image: $RELAY_IMAGE."

: "${TRUSTLESS_SIDECHAIN_IMAGE_TAG:?Variable not set or empty}"
export TRUSTLESS_SIDECHAIN_IMAGE="sidechain-main-cli-docker:$TRUSTLESS_SIDECHAIN_IMAGE_TAG"
echo "Using trustless-sidechain-cli image: $TRUSTLESS_SIDECHAIN_IMAGE."

: "${TESTNET:?Variable not set or empty.}"
export TESTNET="$TESTNET"
echo "Using tesnet: $TESTNET."


echo "Copying signing keys to docker context directory..."
cp ../../$TESTNET/keys/relay/init.skey docker-build-context
cp ../../$TESTNET/keys/relay/relay.skey docker-build-context

export REGISTRY="689191102645.dkr.ecr.eu-central-1.amazonaws.com"
export DOCKER_IMAGE="$REGISTRY/relay:$RELAY_IMAGE_TAG-$TRUSTLESS_SIDECHAIN_IMAGE_TAG-$TESTNET"
echo "Building $DOCKER_IMAGE..."
docker build docker-build-context --build-arg="trustless_sidechain_image=$TRUSTLESS_SIDECHAIN_IMAGE" --build-arg="relay_base_image=$RELAY_IMAGE" -t $DOCKER_IMAGE

echo "Cleaning up docker context directory..."
rm docker-build-context/init.skey docker-build-context/relay.skey

echo "Logging in to ECR..."
aws ecr get-login-password --region eu-central-1 | docker login --username AWS --password-stdin $REGISTRY

echo "Pushing $DOCKER_IMAGE to ECR..."
docker push $DOCKER_IMAGE
