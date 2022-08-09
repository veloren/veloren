#!/bin/bash
VELOREN_ASSETS="$(pwd)/assets"
export VELOREN_ASSETS

time cargo test --package veloren-voxygen-i18n \
    --lib test_all_localizations \
    --features="stat" \
    -- --nocapture --ignored
