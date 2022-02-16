#!/bin/bash
export VELOREN_USERDATA_STRATEGY=executable
time cargo build --release --no-default-features --features default-publish
# evaluate --bin instead, last time i checked (2021-07-14) it was 2 minutes slower on release (but faster on debug)
