FROM debian:stable-slim

# SIGUSR1 causes veloren-server-cli to initiate a graceful shutdown
LABEL com.centurylinklabs.watchtower.stop-signal="SIGUSR1"

ARG PROJECTNAME=server-cli

# librust-backtrace+libbacktrace-dev = backtrace functionality
# iproute2 and net-tools for diagnostic purposes
RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends --assume-yes \
        ca-certificates \
        librust-backtrace+libbacktrace-dev \
        iproute2 \
        net-tools \
    && rm -rf /var/lib/apt/lists/*;

COPY ./veloren-server-cli /opt/veloren-server-cli
COPY ./assets/common /opt/assets/common
COPY ./assets/server /opt/assets/server
COPY ./assets/world /opt/assets/world

WORKDIR /opt

ENV RUST_BACKTRACE=full
ENTRYPOINT ["/opt/veloren-server-cli"]
