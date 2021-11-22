#!/bin/bash
export VELOREN_ASSETS="$(pwd)/assets"
time cargo test --package veloren-voxygen-i18n --lib test_all_localizations -- --nocapture --ignored
