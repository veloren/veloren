#!/bin/bash
VELOREN_ASSETS="$(pwd)/assets"
export VELOREN_ASSETS

time cargo run --bin i18n_csv --features="stat"
