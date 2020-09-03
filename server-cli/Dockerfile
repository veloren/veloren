FROM debian:stable-slim

ARG PROJECTNAME=server-cli

# librust-backtrace+libbacktrace-dev = backtrace functionality
RUN apt-get update; export DEBIAN_FRONTEND=noninteractive; \
    apt-get install -y --no-install-recommends --assume-yes \
        librust-backtrace+libbacktrace-dev; \
    rm -rf /var/lib/apt/lists/*;

COPY ./veloren-server-cli /opt/veloren-server-cli
COPY ./assets/common /opt/assets/common
COPY ./assets/world /opt/assets/world

WORKDIR /opt
CMD [ "sh", "-c", "RUST_BACKTRACE=1 /opt/veloren-server-cli -b" ]
