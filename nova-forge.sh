#!/usr/bin/env bash
# nova-forge.sh — build, run, and test Nova-Forge
# Run './nova-forge.sh help' for full usage.

set -euo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

info()    { echo -e "${GREEN}[nova-forge]${NC} $*"; }
warn()    { echo -e "${YELLOW}[nova-forge] WARN:${NC} $*"; }
error()   { echo -e "${RED}[nova-forge] ERROR:${NC} $*" >&2; }
die()     { error "$*"; exit 1; }
section() { echo -e "\n${BOLD}==> $*${NC}"; }

usage() {
    cat <<'EOF'
nova-forge.sh — build, run, and test Nova-Forge

Usage:
  ./nova-forge.sh <command> [options]

Commands:
  build          Fast dev build of both the client and server (default)
  run            Build & launch the game client (singleplayer / LAN host)
  server         Build & launch the dedicated server (standalone, no auth)
  release        Optimised release build; copies assets beside the binaries
  run-release    Launch the already-built release client
  test           Run the workspace unit-test suite
  clean          Remove all Cargo build artifacts (target/ directory)
  rebuild        Clean all build artifacts then perform a fresh dev build
  help           Show this message

Options (all commands):
  --no-egui            Skip the EGUI debug overlay (faster compile)
  --no-hot             Disable hot-reloading (faster compile)
  --shaderc-from-source  Build shaderc from source (requires cmake + ninja; use
                         only when your system has no pre-built libshaderc)
  --no-default-features  Pass-through to Cargo
  -v, --verbose        Show full Cargo output
  -- <args>            Anything after -- is forwarded directly to Cargo

Environment variables honoured:
  NOVA_FORGE_ASSETS   Path to the assets directory  (default: ./assets)
  RUST_LOG         Tracing filter                (default: info)

Examples:
  ./nova-forge.sh                           # dev build
  ./nova-forge.sh run                       # build & launch client
  ./nova-forge.sh run --no-egui             # launch without debug overlay
  ./nova-forge.sh server                    # run dedicated LAN server
  ./nova-forge.sh release                   # optimised release build (assets copied to target/release/)
  ./nova-forge.sh run-release               # launch the release client
  ./nova-forge.sh test                      # run all workspace tests
  ./nova-forge.sh build --shaderc-from-source  # build shaderc from source (needs ninja)
  ./nova-forge.sh clean                         # wipe all build artifacts
  ./nova-forge.sh rebuild                       # clean then fresh dev build
EOF
    exit 0
}

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
COMMAND="${1:-build}"
shift || true

CARGO_ARGS=()
NO_EGUI=false
NO_HOT=false
SHADERC_FROM_SOURCE=false
VERBOSE=false

# ---------------------------------------------------------------------------
# Parse remaining flags
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-egui)              NO_EGUI=true ;;
        --no-hot)               NO_HOT=true ;;
        --shaderc-from-source)  SHADERC_FROM_SOURCE=true ;;
        --no-default-features)  CARGO_ARGS+=(--no-default-features) ;;
        -v|--verbose)           VERBOSE=true ;;
        --)                     shift; CARGO_ARGS+=("$@"); break ;;
        *)                      CARGO_ARGS+=("$1") ;;
    esac
    shift
done

[[ "$VERBOSE" == true ]] && CARGO_ARGS+=(--verbose)

# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------
export NOVA_FORGE_ASSETS="${NOVA_FORGE_ASSETS:-assets}"
export RUST_LOG="${RUST_LOG:-info}"
# Store saves / config inside the repo tree so running from source doesn't
# pollute the user's home directory.
export NOVA_FORGE_USERDATA_STRATEGY=executable

# ---------------------------------------------------------------------------
# Verify toolchain
# ---------------------------------------------------------------------------
check_toolchain() {
    if ! command -v cargo &>/dev/null; then
        die "cargo not found. Install Rust via https://rustup.rs/"
    fi

    local required_channel
    required_channel=$(cat rust-toolchain 2>/dev/null || true)
    if [[ -n "$required_channel" ]]; then
        if ! rustup toolchain list 2>/dev/null | grep -q "${required_channel%-*}"; then
            warn "Required toolchain '$required_channel' may not be installed."
            warn "Run: rustup toolchain install $required_channel"
        fi
    fi
}

# ---------------------------------------------------------------------------
# Feature flag construction
# ---------------------------------------------------------------------------
# Default dev features: singleplayer + egui + hot-reloading + simd
# shaderc-from-source is NOT included by default; it requires cmake + ninja
# (which many Windows machines lack).  Use --shaderc-from-source to opt in.
build_features() {
    local feats="singleplayer,simd"
    [[ "$NO_EGUI" == false ]] && feats="${feats},egui-ui"
    [[ "$NO_HOT"  == false ]] && feats="${feats},hot-reloading"
    [[ "$SHADERC_FROM_SOURCE" == true ]] && feats="${feats},shaderc-from-source"
    echo "$feats"
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------
cmd_build() {
    local feats
    feats="$(build_features)"
    section "Building Nova-Forge (dev) — client + server"
    info "Features: $feats"
    cargo build \
        --bin nova-forge-voxygen \
        --bin nova-forge-server-cli \
        --features "$feats" \
        "${CARGO_ARGS[@]}"
    info "Build complete."
    info "  Client : target/debug/nova-forge-voxygen"
    info "  Server : target/debug/nova-forge-server-cli"
}

cmd_run() {
    local feats
    feats="$(build_features)"
    section "Building & launching Nova-Forge client"
    info "Features: $feats"
    info "Assets  : $NOVA_FORGE_ASSETS"
    info "Data dir: userdata/ (relative to binary)"
    cargo run \
        --bin nova-forge-voxygen \
        --features "$feats" \
        "${CARGO_ARGS[@]}"
}

cmd_server() {
    section "Building & launching Nova-Forge dedicated server"
    info "No authentication required — any username accepted."
    info "LAN clients can connect to port 14004."
    cargo run \
        --bin nova-forge-server-cli \
        "${CARGO_ARGS[@]}"
}

cmd_release() {
    section "Building Nova-Forge (release) — client + server"
    warn "This can take 10-30 minutes on first build."
    cargo build \
        --release \
        --no-default-features \
        --features default-publish \
        --bin nova-forge-voxygen \
        --bin nova-forge-server-cli \
        "${CARGO_ARGS[@]}"
    info "Release build complete."
    info "  Client : target/release/nova-forge-voxygen"
    info "  Server : target/release/nova-forge-server-cli"

    # Copy the assets directory next to the release binaries so the client can
    # find them when launched directly (e.g. by double-clicking on Windows).
    # We check whether the destination already exists to avoid a redundant
    # 449 MB copy on repeat builds.
    local dest="target/release/assets"
    if [[ -d "$dest" ]]; then
        info "Assets already present at $dest — skipping copy."
    else
        section "Copying assets → $dest"
        cp -r assets "$dest"
        info "Assets copied to $dest."
    fi
    info ""
    info "To run the release client:"
    info "  ./nova-forge.sh run-release"
    info "Or launch the binary directly — assets are now bundled beside it."
}

cmd_run_release() {
    local exe="target/release/nova-forge-voxygen"
    if [[ ! -x "$exe" ]]; then
        die "Release binary not found at $exe. Run './nova-forge.sh release' first."
    fi
    section "Launching Nova-Forge (release client)"
    # NOVA_FORGE_ASSETS is set globally above (default: ./assets in the repo root).
    info "Assets  : $NOVA_FORGE_ASSETS"
    info "Data dir: userdata/ (relative to binary)"
    "$exe"
}

cmd_test() {
    section "Running workspace tests"
    cargo test --workspace "${CARGO_ARGS[@]}"
}

cmd_clean() {
    section "Cleaning build artifacts"
    cargo clean
    info "All build artifacts removed (target/ directory wiped)."
}

cmd_rebuild() {
    cmd_clean
    cmd_build
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------
check_toolchain

case "$COMMAND" in
    build)       cmd_build       ;;
    run)         cmd_run         ;;
    server)      cmd_server      ;;
    release)     cmd_release     ;;
    run-release) cmd_run_release ;;
    test)        cmd_test        ;;
    clean)       cmd_clean       ;;
    rebuild)     cmd_rebuild     ;;
    help|-h|--help) usage ;;
    *) die "Unknown command '$COMMAND'. Run './nova-forge.sh help' for usage." ;;
esac
