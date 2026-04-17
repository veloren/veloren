#!/bin/bash
NOVA_FORGE_ASSETS="$(pwd)/assets";
export NOVA_FORGE_ASSETS;

time cargo test \
    --package nova-forge-common-assets asset_tweak::tests \
    --features asset_tweak --lib &&
time cargo test;
