#!/usr/bin/env bash
# nova-forge.sh — build, run, and test Nova-Forge
#
# Run with NO arguments to get an interactive build-type selection menu.
# Run './nova-forge.sh help' for full command-line usage.

set -euo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

info()    { echo -e "${GREEN}[nova-forge]${NC} $*"; }
warn()    { echo -e "${YELLOW}[nova-forge] WARN:${NC} $*"; }
error()   { echo -e "${RED}[nova-forge] ERROR:${NC} $*" >&2; }
die()     { error "$*"; exit 1; }
section() { echo -e "\n${BOLD}==> $*${NC}"; }

usage() {
    cat <<'EOF'
nova-forge.sh — build, run, and test Nova-Forge

Run with no arguments to open an interactive build-type selection menu.

Usage:
  ./nova-forge.sh [command] [options]

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
  RUST_LOG            Tracing filter                (default: info)

Logging:
  All cargo output is captured to logs/build-<timestamp>.log.
  If a build fails, a Markdown error report is written to
  logs/build-<timestamp>-errors.md — ready to paste into a GitHub issue.

Examples:
  ./nova-forge.sh                           # interactive menu
  ./nova-forge.sh build                     # dev build
  ./nova-forge.sh run                       # build & launch client
  ./nova-forge.sh run --no-egui             # launch without debug overlay
  ./nova-forge.sh server                    # run dedicated LAN server
  ./nova-forge.sh release                   # optimised release build
  ./nova-forge.sh run-release               # launch the release client
  ./nova-forge.sh test                      # run all workspace tests
  ./nova-forge.sh build --shaderc-from-source  # build shaderc from source
  ./nova-forge.sh clean                     # wipe all build artifacts
  ./nova-forge.sh rebuild                   # clean then fresh dev build
EOF
    exit 0
}

# ---------------------------------------------------------------------------
# Interactive build-type selection menu
# Shown when the script is invoked with no arguments.
# ---------------------------------------------------------------------------
interactive_menu() {
    echo -e "\n${BOLD}╔══════════════════════════════════════╗${NC}"
    echo -e "${BOLD}║     Nova-Forge — Build Menu          ║${NC}"
    echo -e "${BOLD}╚══════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${CYAN}1)${NC} Dev build        — fast debug build (client + server)"
    echo -e "  ${CYAN}2)${NC} Run client       — build & launch the game"
    echo -e "  ${CYAN}3)${NC} Server           — build & launch dedicated server"
    echo -e "  ${CYAN}4)${NC} Release build    — optimised build (+ copy assets)"
    echo -e "  ${CYAN}5)${NC} Run release      — launch already-built release client"
    echo -e "  ${CYAN}6)${NC} Test             — run workspace unit tests"
    echo -e "  ${CYAN}7)${NC} Clean            — wipe target/ build artifacts"
    echo -e "  ${CYAN}8)${NC} Rebuild          — clean then fresh dev build"
    echo -e "  ${CYAN}9)${NC} Help             — show full usage"
    echo -e "  ${CYAN}0)${NC} Quit"
    echo ""
    local choice
    read -rp "$(echo -e "${BOLD}Select [0-9]:${NC} ")" choice
    case "$choice" in
        1) COMMAND=build       ;;
        2) COMMAND=run         ;;
        3) COMMAND=server      ;;
        4) COMMAND=release     ;;
        5) COMMAND=run-release ;;
        6) COMMAND=test        ;;
        7) COMMAND=clean       ;;
        8) COMMAND=rebuild     ;;
        9) usage               ;;
        0) exit 0              ;;
        *) die "Invalid selection '$choice'. Run './nova-forge.sh help' for usage." ;;
    esac
}

# ---------------------------------------------------------------------------
# Log / error-capture infrastructure
# ---------------------------------------------------------------------------
LOGS_DIR="logs"
LOG_FILE=""
MD_LOG_FILE=""
_BUILD_CMD_DESC=""

# run_cargo <cargo args...>
#   Runs the given cargo command while:
#     • Teeing all output (stdout + stderr) to a timestamped log file.
#     • On failure: writing a Markdown error report and showing a summary
#       that must be confirmed (Enter) before the terminal closes.
run_cargo() {
    mkdir -p "$LOGS_DIR"
    local ts
    ts="$(date +%Y%m%d-%H%M%S)"
    LOG_FILE="${LOGS_DIR}/build-${ts}.log"
    MD_LOG_FILE="${LOGS_DIR}/build-${ts}-errors.md"
    _BUILD_CMD_DESC="$*"

    info "Log file : $LOG_FILE"

    local exit_code=0
    # Temporarily disable errexit so the exit code can be read through the pipe.
    set +e
    "$@" 2>&1 | tee "$LOG_FILE"
    exit_code="${PIPESTATUS[0]}"
    set -e

    if [[ "$exit_code" -ne 0 ]]; then
        _write_md_log "$exit_code"
        _show_error_summary "$exit_code"
        # Keep the terminal open so the user can read the summary.
        read -rp "$(echo -e "${BOLD}Press ENTER to close...${NC}")" _
        exit "$exit_code"
    fi
}

# _show_error_summary <exit_code>
#   Prints a formatted error summary to the terminal using the captured log.
_show_error_summary() {
    local exit_code="$1"

    echo ""
    echo -e "${RED}${BOLD}╔══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}${BOLD}║              BUILD FAILED — ERROR SUMMARY                ║${NC}"
    echo -e "${RED}${BOLD}╚══════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Rust compiler error headlines (error[Exxxx]: …) and file locations (-->).
    local err_lines loc_lines error_count
    err_lines="$(grep -E '^error(\[E[0-9]+\])?:' "$LOG_FILE" 2>/dev/null || true)"
    loc_lines="$(grep -E '^ *-->' "$LOG_FILE" 2>/dev/null || true)"
    error_count="$(echo "$err_lines" | grep -c '^error' 2>/dev/null || true)"
    # grep -c returns 1 when there are no matches; treat that as 0.
    [[ "$error_count" =~ ^[0-9]+$ ]] || error_count=0

    if [[ -n "$err_lines" ]]; then
        echo -e "${YELLOW}Compiler errors:${NC}"
        echo "$err_lines"
        echo ""
        if [[ -n "$loc_lines" ]]; then
            echo -e "${YELLOW}Source locations:${NC}"
            echo "$loc_lines"
            echo ""
        fi
    else
        echo -e "${YELLOW}No structured compiler errors detected — see full log for details.${NC}"
        echo ""
    fi

    echo -e "  ${RED}Total errors : $error_count${NC}"
    echo -e "  Exit code    : $exit_code"
    echo -e "  Full log     : ${CYAN}${LOG_FILE}${NC}"
    echo -e "  Error report : ${CYAN}${MD_LOG_FILE}${NC}"
    echo ""
    echo -e "${YELLOW}The Markdown report above can be pasted directly into a GitHub issue or chat.${NC}"
    echo ""
}

# _write_md_log <exit_code>
#   Writes a pasteable Markdown error report to $MD_LOG_FILE.
_write_md_log() {
    local exit_code="$1"
    local date_str
    date_str="$(date '+%Y-%m-%d %H:%M:%S')"

    # Grab each error block: headline + up to 10 following lines of context.
    local error_blocks
    error_blocks="$(grep -E -A 10 '^error(\[E[0-9]+\])?:' "$LOG_FILE" 2>/dev/null \
        | grep -v '^--$' \
        || true)"

    local error_count
    error_count="$(grep -cE '^error(\[E[0-9]+\])?:' "$LOG_FILE" 2>/dev/null || true)"
    [[ "$error_count" =~ ^[0-9]+$ ]] || error_count=0

    cat > "$MD_LOG_FILE" <<MARKDOWN
# Nova-Forge Build Error Report

| Field         | Value |
|---------------|-------|
| **Date**      | ${date_str} |
| **Command**   | \`${_BUILD_CMD_DESC}\` |
| **Exit code** | ${exit_code} |
| **Errors**    | ${error_count} |

## Error Summary

\`\`\`
${error_blocks}
\`\`\`

## Reproduce

\`\`\`bash
${_BUILD_CMD_DESC}
\`\`\`

## Full log

See: \`${LOG_FILE}\`

---
*Generated by nova-forge.sh on ${date_str}*
MARKDOWN

    info "Markdown error report → $MD_LOG_FILE"
}

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
COMMAND="${1:-}"
if [[ -z "$COMMAND" ]]; then
    interactive_menu
else
    shift || true
fi

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
    run_cargo cargo build \
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
    run_cargo cargo run \
        --bin nova-forge-voxygen \
        --features "$feats" \
        "${CARGO_ARGS[@]}"
}

cmd_server() {
    section "Building & launching Nova-Forge dedicated server"
    info "No authentication required — any username accepted."
    info "LAN clients can connect to port 14004."
    run_cargo cargo run \
        --bin nova-forge-server-cli \
        "${CARGO_ARGS[@]}" \
        -- --no-auth
}

cmd_release() {
    section "Building Nova-Forge (release) — client + server"
    warn "This can take 10-30 minutes on first build."
    run_cargo cargo build \
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
    run_cargo cargo test --workspace "${CARGO_ARGS[@]}"
}

cmd_clean() {
    section "Cleaning build artifacts"
    run_cargo cargo clean
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
