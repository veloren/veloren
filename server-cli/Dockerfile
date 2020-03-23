FROM debian:stable-slim

ARG PROJECTNAME=server-cli

COPY ./veloren-server-cli /opt/veloren-server-cli
COPY ./assets/common /opt/assets/common
COPY ./assets/world /opt/assets/world

WORKDIR /opt
CMD [ "sh", "-c", "RUST_LOG=info,common=debug,common::net=info RUST_BACKTRACE=1 /opt/veloren-server-cli" ]
