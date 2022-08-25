#!/bin/bash
VELOREN_ASSETS="$(pwd)/assets"
export VELOREN_ASSETS

time cargo test \
    --package veloren-common-assets asset_tweak::tests \
    --features asset_tweak --lib &&
time cargo test
