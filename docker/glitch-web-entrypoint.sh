#!/usr/bin/env bash
set -euo pipefail

log() { printf '[glitch-web] %s\n' "$*"; }
warn() { printf '[glitch-web] WARNING: %s\n' "$*"; }
fail() { printf '[glitch-web] ERROR: %s\n' "$*" >&2; exit 1; }

: "${GLITCH_TITLE_ID:?GLITCH_TITLE_ID is required}"
: "${GLITCH_TITLE_TOKEN:?GLITCH_TITLE_TOKEN is required}"
: "${GLITCH_SHARED_PASSWORD:?GLITCH_SHARED_PASSWORD is required}"
: "${GLITCH_INSTALL_ID:?GLITCH_INSTALL_ID is required for streamed web play}"

ROOT_USERDATA="${VELOREN_USERDATA:-/opt/userdata}"
WEB_MODE="${VELOREN_WEB_MODE:-all_in_one}" # all_in_one | client_only | server_only
WEB_PORT="${VELOREN_WEB_PORT:-6080}"
VNC_PORT="${VELOREN_VNC_PORT:-5900}"
DEPTH="${VELOREN_WEB_DEPTH:-24}"
SERVER_HOST="${VELOREN_SERVER_HOST:-127.0.0.1}"
SERVER_PORT="${VELOREN_SERVER_PORT:-14004}"
DISPLAY="${DISPLAY:-:99}"
SERVER_READY_TIMEOUT="${VELOREN_SERVER_READY_TIMEOUT:-600}"
SERVER_GRACE_SECONDS="${VELOREN_SERVER_GRACE_SECONDS:-0}"
SERVER_READY_PATTERN="${VELOREN_SERVER_READY_PATTERN:-Server is ready to accept connections}"
STREAM_READY_TIMEOUT="${VELOREN_STREAM_READY_TIMEOUT:-60}"
STREAM_PRESET="${VELOREN_STREAM_PRESET:-balanced}" # performance | balanced | quality | custom
AUTH_MODE="${VELOREN_AUTH_MODE:-glitch}" # glitch | official | custom | none
AUTH_AUTOREGISTER="${VELOREN_AUTH_AUTOREGISTER:-0}"

case "$STREAM_PRESET" in
  performance)
    : "${VELOREN_WEB_WIDTH:=960}"
    : "${VELOREN_WEB_HEIGHT:=540}"
    : "${VELOREN_VNC_QUALITY:=6}"
    : "${VELOREN_NOVNC_QUALITY:=6}"
    : "${VELOREN_VNC_COMPRESS_LEVEL:=2}"
    : "${VELOREN_NOVNC_COMPRESSION:=2}"
    : "${VELOREN_VNC_WAIT_MS:=16}"
    : "${VELOREN_VNC_DEFER_MS:=16}"
    ;;
  quality)
    : "${VELOREN_WEB_WIDTH:=1600}"
    : "${VELOREN_WEB_HEIGHT:=900}"
    : "${VELOREN_VNC_QUALITY:=9}"
    : "${VELOREN_NOVNC_QUALITY:=9}"
    : "${VELOREN_VNC_COMPRESS_LEVEL:=1}"
    : "${VELOREN_NOVNC_COMPRESSION:=1}"
    : "${VELOREN_VNC_WAIT_MS:=8}"
    : "${VELOREN_VNC_DEFER_MS:=8}"
    ;;
  balanced|custom)
    : "${VELOREN_WEB_WIDTH:=1280}"
    : "${VELOREN_WEB_HEIGHT:=720}"
    : "${VELOREN_VNC_QUALITY:=8}"
    : "${VELOREN_NOVNC_QUALITY:=8}"
    : "${VELOREN_VNC_COMPRESS_LEVEL:=1}"
    : "${VELOREN_NOVNC_COMPRESSION:=1}"
    : "${VELOREN_VNC_WAIT_MS:=10}"
    : "${VELOREN_VNC_DEFER_MS:=10}"
    ;;
  *)
    fail "Unsupported VELOREN_STREAM_PRESET='${STREAM_PRESET}'. Use performance, balanced, quality, or custom."
    ;;
esac

WIDTH="${VELOREN_WEB_WIDTH}"
HEIGHT="${VELOREN_WEB_HEIGHT}"
VNC_QUALITY="${VELOREN_VNC_QUALITY}"
NOVNC_QUALITY="${VELOREN_NOVNC_QUALITY}"
VNC_COMPRESS_LEVEL="${VELOREN_VNC_COMPRESS_LEVEL}"
NOVNC_COMPRESSION="${VELOREN_NOVNC_COMPRESSION}"
VNC_WAIT_MS="${VELOREN_VNC_WAIT_MS}"
VNC_DEFER_MS="${VELOREN_VNC_DEFER_MS}"
VNC_NCACHE="${VELOREN_VNC_NCACHE:-0}"
BROWSER_RESIZE_MODE="${VELOREN_BROWSER_RESIZE_MODE:-scale}"
VNC_EXTRA_ARGS="${VELOREN_VNC_EXTRA_ARGS:-}"

SERVER_USERDATA="${VELOREN_SERVER_USERDATA:-${ROOT_USERDATA}/server-runtime}"
CLIENT_USERDATA="${VELOREN_CLIENT_USERDATA:-${ROOT_USERDATA}/client-runtime}"
LOG_DIR="${VELOREN_WEB_LOG_DIR:-/tmp/veloren-web}"
WEB_HOME="${VELOREN_WEB_HOME:-/tmp/veloren-home}"
AUTH_ENV_FILE="${VELOREN_AUTH_ENV_FILE:-${LOG_DIR}/veloren-auth.env}"
FLUXBOX_INIT="${WEB_HOME}/.fluxbox/init"

BROWSER_URL_PATH="/vnc.html?autoconnect=1&resize=${BROWSER_RESIZE_MODE}&quality=${NOVNC_QUALITY}&compression=${NOVNC_COMPRESSION}&shared=1"

export DISPLAY
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/xdg-runtime}"
export VELOREN_GLITCH_AUTOCONNECT="${VELOREN_GLITCH_AUTOCONNECT:-1}"
export VELOREN_SERVER_ADDRESS="${VELOREN_SERVER_ADDRESS:-${SERVER_HOST}:${SERVER_PORT}}"
export HOME="${WEB_HOME}"
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"
export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"

mkdir -p "$ROOT_USERDATA" "$SERVER_USERDATA" "$CLIENT_USERDATA" "$XDG_RUNTIME_DIR" "$LOG_DIR" "$WEB_HOME/.fluxbox"
chmod 700 "$XDG_RUNTIME_DIR" "$WEB_HOME" || true

show_debug_logs() {
  log "--- debug logs ---"
  for file in \
    "$LOG_DIR/server.log" \
    "$LOG_DIR/voxygen.log" \
    "$LOG_DIR/xvfb.log" \
    "$LOG_DIR/fluxbox.log" \
    "$LOG_DIR/pulseaudio.log" \
    "$LOG_DIR/x11vnc.log" \
    "$LOG_DIR/websockify.log" \
    "$LOG_DIR/veloren-auth-provisioner.log"; do
    if [[ -f "$file" ]]; then
      log "--- tail ${file} ---"
      tail -n 160 "$file" || true
    fi
  done
}

check_runtime_library() {
  local lib="$1"
  if command -v ldconfig >/dev/null 2>&1 && ldconfig -p 2>/dev/null | grep -q "${lib}"; then
    return 0
  fi
  if find /lib /usr/lib -name "${lib}*" -print -quit 2>/dev/null | grep -q .; then
    return 0
  fi
  fail "Missing runtime library ${lib}. Rebuild with the runtime dependency Dockerfile."
}

check_stream_runtime_dependencies() {
  log "Checking streamed-client runtime libraries"
  check_runtime_library "libxkbcommon-x11.so"
  check_runtime_library "libxkbcommon.so"
  check_runtime_library "libX11.so"
  check_runtime_library "libxcb.so"
  check_runtime_library "libvulkan.so"
  check_runtime_library "libGL.so"
}

disable_fluxbox_wallpaper_popup() {
  # Fluxbox may call fbsetbg, and fbsetbg can open an xmessage popup in the VNC
  # desktop saying it cannot find a wallpaper setter. In a streamed game session,
  # wallpaper management is unwanted, so make fbsetbg a harmless no-op.
  if [[ -x /usr/bin/fbsetbg && ! -f /usr/bin/fbsetbg.real ]]; then
    mv /usr/bin/fbsetbg /usr/bin/fbsetbg.real || true
    cat > /usr/bin/fbsetbg <<'SH'
#!/bin/sh
exit 0
SH
    chmod +x /usr/bin/fbsetbg || true
  fi

  # If anything still tries to open xmessage, make it harmless too.
  if ! command -v xmessage >/dev/null 2>&1; then
    cat > /usr/local/bin/xmessage <<'SH'
#!/bin/sh
exit 0
SH
    chmod +x /usr/local/bin/xmessage || true
    export PATH="/usr/local/bin:${PATH}"
  fi
}

create_fluxbox_config() {
  mkdir -p "$WEB_HOME/.fluxbox"
  cat > "$FLUXBOX_INIT" <<EOF2
session.screen0.rootCommand: xsetroot -solid black
session.screen0.toolbar.visible: false
session.screen0.workspaces: 1
session.screen0.fullMaximization: true
session.screen0.defaultDeco: NONE
session.screen0.windowPlacement: RowSmartPlacement
session.menuFile: $WEB_HOME/.fluxbox/menu
session.keyFile: $WEB_HOME/.fluxbox/keys
session.styleOverlay: $WEB_HOME/.fluxbox/overlay
session.configVersion: 13
EOF2
  : > "$WEB_HOME/.fluxbox/menu"
  : > "$WEB_HOME/.fluxbox/keys"
  : > "$WEB_HOME/.fluxbox/overlay"
}

wait_for_http() {
  local url="$1" timeout_s="${2:-60}" start
  start="$(date +%s)"
  while true; do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    if (( $(date +%s) - start > timeout_s )); then
      log "Timed out waiting for HTTP ${url}"
      return 1
    fi
    sleep 1
  done
}

wait_for_tcp() {
  local host="$1" port="$2" timeout_s="${3:-120}" start
  start="$(date +%s)"
  while true; do
    if timeout 1 bash -lc "</dev/tcp/${host}/${port}" >/dev/null 2>&1; then
      return 0
    fi
    if (( $(date +%s) - start > timeout_s )); then
      log "Timed out waiting for TCP ${host}:${port}"
      return 1
    fi
    sleep 1
  done
}

wait_for_log() {
  local file="$1" pattern="$2" timeout_s="${3:-120}" start
  start="$(date +%s)"
  while true; do
    if [[ -f "$file" ]] && grep -q "$pattern" "$file"; then
      return 0
    fi
    if [[ -f "$file" ]] && grep -q "Tried to fetch resource of type.*Arc<World>" "$file"; then
      log "Detected Veloren missing Arc<World> panic. The server was probably built without default worldgen features."
      show_debug_logs
      return 1
    fi
    if [[ -n "${SERVER_PID:-}" ]] && ! kill -0 "$SERVER_PID" 2>/dev/null; then
      log "Server process exited before readiness pattern appeared: ${pattern}"
      show_debug_logs
      return 1
    fi
    if (( $(date +%s) - start > timeout_s )); then
      log "Timed out waiting for log pattern '${pattern}' in ${file}"
      show_debug_logs
      return 1
    fi
    sleep 1
  done
}

check_process_alive() {
  local pid="$1" name="$2"
  if ! kill -0 "$pid" 2>/dev/null; then
    log "${name} exited unexpectedly"
    show_debug_logs
    return 1
  fi
}

start_x11vnc_minimal() {
  log "Starting x11vnc minimal fallback on localhost:${VNC_PORT}"
  x11vnc -display "$DISPLAY" \
    -localhost \
    -forever \
    -shared \
    -nopw \
    -noxdamage \
    -rfbport "$VNC_PORT" \
    > "$LOG_DIR/x11vnc.log" 2>&1 &
  X11VNC_PID=$!
}

start_x11vnc_tuned() {
  log "Starting x11vnc tuned on localhost:${VNC_PORT}"

  local args=(
    -display "$DISPLAY"
    -localhost
    -forever
    -shared
    -nopw
    -noxdamage
    -rfbport "$VNC_PORT"
    -quality "$VNC_QUALITY"
    -compresslevel "$VNC_COMPRESS_LEVEL"
    -wait "$VNC_WAIT_MS"
    -defer "$VNC_DEFER_MS"
  )

  # Do not pass -ncache 0. Some x11vnc builds treat client-side caching badly
  # in browser/noVNC paths. Only enable it when explicitly set above zero.
  if [[ "$VNC_NCACHE" =~ ^[0-9]+$ ]] && (( VNC_NCACHE > 0 )); then
    args+=( -ncache "$VNC_NCACHE" )
  fi

  if [[ -n "$VNC_EXTRA_ARGS" ]]; then
    # shellcheck disable=SC2206
    local extra=( $VNC_EXTRA_ARGS )
    args+=( "${extra[@]}" )
  fi

  x11vnc "${args[@]}" > "$LOG_DIR/x11vnc.log" 2>&1 &
  X11VNC_PID=$!
}

start_x11vnc_robust() {
  : > "$LOG_DIR/x11vnc.log"

  start_x11vnc_tuned

  if wait_for_tcp 127.0.0.1 "$VNC_PORT" 12; then
    check_process_alive "$X11VNC_PID" "x11vnc"
    return 0
  fi

  log "Tuned x11vnc did not open port ${VNC_PORT}; trying minimal fallback."
  if [[ -n "${X11VNC_PID:-}" ]]; then
    kill "$X11VNC_PID" 2>/dev/null || true
    sleep 1
  fi

  log "--- failed tuned x11vnc log ---"
  tail -n 120 "$LOG_DIR/x11vnc.log" || true

  : > "$LOG_DIR/x11vnc.log"
  start_x11vnc_minimal

  wait_for_tcp 127.0.0.1 "$VNC_PORT" "$STREAM_READY_TIMEOUT"
  check_process_alive "$X11VNC_PID" "x11vnc"
}

cleanup() {
  log "Stopping streamed native session"
  jobs -p | xargs -r kill 2>/dev/null || true
}
trap cleanup EXIT INT TERM

log "=== Veloren Glitch Streamed Web Session ==="
log "Title: ${GLITCH_TITLE_ID}"
log "Mode: ${WEB_MODE}"
log "Browser URL: http://localhost:${WEB_PORT}${BROWSER_URL_PATH}"
log "Veloren server: ${VELOREN_SERVER_ADDRESS}"
log "Display: ${DISPLAY} ${WIDTH}x${HEIGHT}x${DEPTH}"
log "Stream preset: ${STREAM_PRESET}"
log "VNC quality/compression: quality=${VNC_QUALITY}, compress=${VNC_COMPRESS_LEVEL}, wait=${VNC_WAIT_MS}ms, defer=${VNC_DEFER_MS}ms, ncache=${VNC_NCACHE}"
log "noVNC query: quality=${NOVNC_QUALITY}, compression=${NOVNC_COMPRESSION}, resize=${BROWSER_RESIZE_MODE}"
log "Renderer mode: LIBGL_ALWAYS_SOFTWARE=${LIBGL_ALWAYS_SOFTWARE:-unset}, WGPU_BACKEND=${WGPU_BACKEND:-unset}"
log "Root userdata: ${ROOT_USERDATA}"
log "Server userdata: ${SERVER_USERDATA}"
log "Client userdata: ${CLIENT_USERDATA}"
log "Log dir: ${LOG_DIR}"
log "Window-manager home: ${WEB_HOME}"
log "Veloren auth mode: ${AUTH_MODE}"
log "Veloren auth auto-register: ${AUTH_AUTOREGISTER}"

case "$WEB_MODE" in
  all_in_one|client_only|server_only) ;;
  *) fail "Unsupported VELOREN_WEB_MODE='${WEB_MODE}'. Use all_in_one, client_only, or server_only." ;;
esac

if [[ "$GLITCH_TITLE_TOKEN" == YOUR_* || "$GLITCH_INSTALL_ID" == YOUR_* ]]; then
  warn "Placeholder Glitch credentials detected. The stream may open, but login/validation will fail until real values are supplied."
fi

if [[ "$WEB_MODE" != "server_only" ]]; then
  check_stream_runtime_dependencies
fi

if [[ "$WEB_MODE" == "server_only" ]]; then
  log "Server-only mode requested; starting dedicated Veloren server"
  export VELOREN_USERDATA="${SERVER_USERDATA}"
  exec /usr/local/bin/glitch-entrypoint.sh
fi

if [[ "$AUTH_MODE" == "official" || "$AUTH_MODE" == "custom" ]]; then
  if [[ "$AUTH_AUTOREGISTER" == "1" ]]; then
    log "Auto-provisioning Veloren auth account from Glitch install"
    /usr/local/bin/veloren-auth-provisioner --env-file "$AUTH_ENV_FILE" \
      > "$LOG_DIR/veloren-auth-provisioner.log" 2>&1 || {
        log "Veloren auth auto-provisioning failed"
        cat "$LOG_DIR/veloren-auth-provisioner.log" || true
        exit 1
      }

    # shellcheck disable=SC1090
    source "$AUTH_ENV_FILE"
    export VELOREN_USERNAME VELOREN_PASSWORD VELOREN_GLITCH_ORIGINAL_INSTALL_ID VELOREN_GLITCH_DISPLAY_NAME

    export GLITCH_INSTALL_ID="${VELOREN_USERNAME}"
    export GLITCH_SHARED_PASSWORD="${VELOREN_PASSWORD}"

    log "Auto-provisioned Veloren username: ${VELOREN_USERNAME} from Glitch install ${VELOREN_GLITCH_ORIGINAL_INSTALL_ID}"
  else
    log "Using official/custom Veloren auth without auto-register. Expect VELOREN_USERNAME and VELOREN_PASSWORD to be supplied."
    if [[ -n "${VELOREN_USERNAME:-}" && -n "${VELOREN_PASSWORD:-}" ]]; then
      export GLITCH_INSTALL_ID="${VELOREN_USERNAME}"
      export GLITCH_SHARED_PASSWORD="${VELOREN_PASSWORD}"
    fi
  fi
fi

log "Starting virtual X display"
Xvfb "$DISPLAY" -screen 0 "${WIDTH}x${HEIGHT}x${DEPTH}" -nolisten tcp \
  > "$LOG_DIR/xvfb.log" 2>&1 &
XVFB_PID=$!
sleep 1
check_process_alive "$XVFB_PID" "Xvfb"

disable_fluxbox_wallpaper_popup

log "Starting window manager"
create_fluxbox_config
fluxbox -rc "$FLUXBOX_INIT" > "$LOG_DIR/fluxbox.log" 2>&1 &
FLUXBOX_PID=$!
sleep 1
check_process_alive "$FLUXBOX_PID" "fluxbox"

log "Starting PulseAudio if available"
pulseaudio --start --exit-idle-time=-1 > "$LOG_DIR/pulseaudio.log" 2>&1 || true

start_x11vnc_robust

log "Starting noVNC/websockify on 0.0.0.0:${WEB_PORT}"
websockify --web=/usr/share/novnc/ "0.0.0.0:${WEB_PORT}" "127.0.0.1:${VNC_PORT}" \
  > "$LOG_DIR/websockify.log" 2>&1 &
WEBSOCKIFY_PID=$!
wait_for_http "http://127.0.0.1:${WEB_PORT}/vnc.html" "$STREAM_READY_TIMEOUT"
check_process_alive "$WEBSOCKIFY_PID" "websockify"
log "Stream endpoint is up: ${BROWSER_URL_PATH}"

if [[ "$WEB_MODE" == "all_in_one" ]]; then
  log "Starting local Veloren dedicated server in background"
  env VELOREN_USERDATA="${SERVER_USERDATA}" /usr/local/bin/glitch-entrypoint.sh \
    > "$LOG_DIR/server.log" 2>&1 &
  SERVER_PID=$!
  log "Server PID: ${SERVER_PID}"

  wait_for_tcp 127.0.0.1 "$SERVER_PORT" "$SERVER_READY_TIMEOUT"
  wait_for_log "$LOG_DIR/server.log" "$SERVER_READY_PATTERN" "$SERVER_READY_TIMEOUT"

  log "Server reported ready; waiting ${SERVER_GRACE_SECONDS}s before launching client"
  sleep "$SERVER_GRACE_SECONDS"
  check_process_alive "$SERVER_PID" "Veloren dedicated server"
else
  log "Client-only mode; waiting for external server ${SERVER_HOST}:${SERVER_PORT}"
  wait_for_tcp "$SERVER_HOST" "$SERVER_PORT" "$SERVER_READY_TIMEOUT"
  log "External server TCP is reachable; waiting ${SERVER_GRACE_SECONDS}s before launching client"
  sleep "$SERVER_GRACE_SECONDS"
fi

log "Starting Veloren Voxygen client"
log "Login username: ${GLITCH_INSTALL_ID}"
env VELOREN_USERDATA="${CLIENT_USERDATA}" /opt/veloren-voxygen --server "${VELOREN_SERVER_ADDRESS}" \
  > "$LOG_DIR/voxygen.log" 2>&1 &
CLIENT_PID=$!

log "Browser stream is ready. Open: http://localhost:${WEB_PORT}${BROWSER_URL_PATH}"
log "Tailing client/server logs."

tail -n +1 -F "$LOG_DIR/voxygen.log" "$LOG_DIR/server.log" 2>/dev/null &
TAIL_PID=$!

sleep 8
if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
  log "Voxygen exited during startup."
  show_debug_logs
  wait "$CLIENT_PID"
fi

wait "$CLIENT_PID"
