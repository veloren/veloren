#!/bin/bash
VELOREN_ASSETS="$(pwd)/assets";
export VELOREN_ASSETS;

time cargo test;
