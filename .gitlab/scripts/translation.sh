#!/bin/bash
NOVA_FORGE_ASSETS="$(pwd)/assets";
export NOVA_FORGE_ASSETS;

time cargo run --bin i18n_csv --features="stat";
