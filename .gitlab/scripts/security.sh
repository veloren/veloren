#!/bin/bash
# RUSTSEC-2021-0119: out-of-bounds write in nix::unistd::getgrouplist in a old nix version (0.18 and 0.20) that are needed by old winit,
time cargo audit --ignore RUSTSEC-2021-0119