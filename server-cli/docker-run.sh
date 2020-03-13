#!/bin/sh
cd /opt
RUST_LOG=info,common=debug,common::net=info RUST_BACKTRACE=1 /opt/veloren-server-cli
