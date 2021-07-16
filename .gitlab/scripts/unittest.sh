#!/bin/bash
export VELOREN_ASSETS="$(pwd)/assets"
time cargo test --package veloren-i18n --lib test_all_localizations -- --nocapture --ignored &&
time cargo test --package veloren-common-assets asset_tweak::tests --features asset_tweak --lib &&
time cargo test