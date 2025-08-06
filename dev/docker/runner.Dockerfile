FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
	ca-certificates \
	libgcc-s1 \
	libstdc++6 \
	libc6 \
	libssl3 \
	zlib1g \
	libgomp1 \
	curl \
	wget \
	vim \
	dnsutils \
	jq \
	htop \
	ncat \
	expect \
	xxd

RUN useradd -m -u 1010 -U -s /bin/sh -d /substrate substrate \
	&& mkdir -p /data /substrate/.local/share/partner-chains-node \
	&& chown -R substrate:substrate /data /substrate \
	&& ln -s /data /substrate/.local/share/partner-chains-node
