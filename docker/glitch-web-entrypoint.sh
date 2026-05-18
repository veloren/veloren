#!/usr/bin/env bash
# GLITCH_USER_PULSEAUDIO_AUDIO_V1:
# Run PulseAudio, ffmpeg audio capture, and the Voxygen audio client against a
# normal non-root PulseAudio user daemon. Do not use pulseaudio --system.
export GLITCH_RUNTIME_USER="${GLITCH_RUNTIME_USER:-glitch}"
export GLITCH_RUNTIME_HOME="${GLITCH_RUNTIME_HOME:-/home/glitch}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}"
export PULSE_RUNTIME_PATH="${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}"
export PULSE_SERVER="${PULSE_SERVER:-unix:${PULSE_RUNTIME_PATH}/native}"
export PULSE_SINK="${PULSE_SINK:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}}"
export PULSE_SOURCE="${PULSE_SOURCE:-${GLITCH_AUDIO_SOURCE:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}.monitor}}"
export SDL_AUDIODRIVER="${SDL_AUDIODRIVER:-pulse}"
export ALSOFT_DRIVERS="${ALSOFT_DRIVERS:-pulse}"

# GLITCH_CONTAINER_AUDIO_PROXY_V1C:
# Fix local/prod safer audio proxy:
# - public container port 6080 belongs to nginx
# - noVNC/websockify binds internal 6082
# - PulseAudio starts with an explicit root-container-safe config
export GLITCH_PUBLIC_PORT="${GLITCH_PUBLIC_PORT:-6080}"
export GLITCH_NOVNC_INTERNAL_PORT="${GLITCH_NOVNC_INTERNAL_PORT:-6082}"
export GLITCH_AUDIO_PORT="${GLITCH_AUDIO_PORT:-6081}"

# GLITCH_CONTAINER_AUDIO_PROXY_V1B:
# Fixes safer container-scoped proxy: noVNC/websockify must bind internally
# on 6082, nginx owns public 6080, and PulseAudio must support root containers.
export GLITCH_PUBLIC_PORT="${GLITCH_PUBLIC_PORT:-6080}"
export GLITCH_NOVNC_INTERNAL_PORT="${GLITCH_NOVNC_INTERNAL_PORT:-6082}"
export GLITCH_AUDIO_PORT="${GLITCH_AUDIO_PORT:-6081}"
# GLITCH_CONTAINER_AUDIO_PROXY_V1:
# Production-safe audio routing without matchmaker changes:
#   VM Caddy/proxy -> container :6080
#   container nginx :${GLITCH_NOVNC_INTERNAL_PORT:-6082} -> noVNC/websockify :6082
#   container nginx :6080 -> audio streamer :6081 for /glitch-audio*
export GLITCH_PUBLIC_PORT="${GLITCH_PUBLIC_PORT:-6080}"
export GLITCH_NOVNC_INTERNAL_PORT="${GLITCH_NOVNC_INTERNAL_PORT:-6082}"

# GLITCH_STREAMED_NATIVE_AUDIO_V1:
# Adds browser audio for streamed-native/noVNC by capturing game audio from a
# PulseAudio null sink and serving WebM/Opus through /glitch-audio.webm.
export GLITCH_AUDIO_ENABLED="${GLITCH_AUDIO_ENABLED:-1}"
export GLITCH_AUDIO_PORT="${GLITCH_AUDIO_PORT:-6081}"
export GLITCH_AUDIO_SINK="${GLITCH_AUDIO_SINK:-glitch_stream_sink}"
export GLITCH_AUDIO_BITRATE="${GLITCH_AUDIO_BITRATE:-96000}"
export GLITCH_AUDIO_SAMPLE_RATE="${GLITCH_AUDIO_SAMPLE_RATE:-48000}"
export GLITCH_AUDIO_CHANNELS="${GLITCH_AUDIO_CHANNELS:-2}"

# GLITCH_MOUSE_V1_LITE_RESTORE: restore first-generation noVNC camera mapping; keep splash/save.
# GLITCH_REVERT_NOVNC_MOUSE_OPTIMAL_V1: disable fake noVNC camera pan; keep splash/save.
set -euo pipefail
# GLITCH_STREAM_CAMERA_SAVE_SPLASH_V3:
# Browser-streamed mouse tuning. Desktop Veloren is untouched; these only apply
# when GLITCH_VNC_ABSOLUTE_MOUSE=1 inside noVNC.
export GLITCH_VNC_ABSOLUTE_MOUSE="${GLITCH_VNC_ABSOLUTE_MOUSE:-1}"
export GLITCH_VNC_ABSOLUTE_MOUSE_MAX_DELTA="${GLITCH_VNC_ABSOLUTE_MOUSE_MAX_DELTA:-48}"
export GLITCH_VNC_ABSOLUTE_MOUSE_MAX_Y_DELTA="${GLITCH_VNC_ABSOLUTE_MOUSE_MAX_Y_DELTA:-28}"
export GLITCH_VNC_ABSOLUTE_MOUSE_DEADZONE="${GLITCH_VNC_ABSOLUTE_MOUSE_DEADZONE:-1.8}"
export GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE="${GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE:-0.12}"
export GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE="${GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE:-0.035}"
export VELOREN_CLIENT_START_DELAY_SECONDS="${VELOREN_CLIENT_START_DELAY_SECONDS:-8}"

# Glitch Cloud Save. Slot 0 is the whole Veloren streamed-native userdata archive.
export GLITCH_CLOUD_SAVE_ENABLED="${GLITCH_CLOUD_SAVE_ENABLED:-1}"
export GLITCH_CLOUD_SAVE_SLOT="${GLITCH_CLOUD_SAVE_SLOT:-0}"
export GLITCH_CLOUD_SAVE_INTERVAL_SECONDS="${GLITCH_CLOUD_SAVE_INTERVAL_SECONDS:-60}"
export GLITCH_CLOUD_SAVE_MAX_BYTES="${GLITCH_CLOUD_SAVE_MAX_BYTES:-9500000}"



log() { printf '[glitch-web] %s\n' "$*"; }
warn() { printf '[glitch-web] WARNING: %s\n' "$*"; }
fail() { printf '[glitch-web] ERROR: %s\n' "$*" >&2; exit 1; }

: "${GLITCH_TITLE_ID:?GLITCH_TITLE_ID is required}"
: "${GLITCH_TITLE_TOKEN:?GLITCH_TITLE_TOKEN is required}"
: "${GLITCH_SHARED_PASSWORD:?GLITCH_SHARED_PASSWORD is required}"

ROOT_USERDATA="${VELOREN_USERDATA:-/opt/userdata}"
WEB_MODE="${VELOREN_WEB_MODE:-all_in_one}" # all_in_one | client_only | server_only

if [[ "${WEB_MODE}" != "server_only" ]]; then
  # GLITCH_IDLE_LAUNCHER_PATCH_V1:
# Do not require GLITCH_INSTALL_ID at container boot for streamed web play.
# The VM/container is allowed to come up idle, serve noVNC, then start
# Veloren after the iframe URL supplies install_id/session_id.
if [[ "${VELOREN_WEB_MODE:-all_in_one}" == "server_only" ]]; then
  : "${GLITCH_INSTALL_ID:=}"
else
  : "${GLITCH_INSTALL_ID:=}"
fi
fi
WEB_PORT="${GLITCH_NOVNC_INTERNAL_PORT:-6082}"
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
GPU_MODE="${VELOREN_ENABLE_GPU:-auto}" # auto | 1 | true | 0 | false

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
gpu_device_available() {
  [[ -e /dev/nvidia0 ]] && return 0
  if command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

gpu_requested() {
  case "${GPU_MODE}" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    0|false|FALSE|no|NO|off|OFF) return 1 ;;
    auto|AUTO|"") gpu_device_available ; return $? ;;
    *) gpu_device_available ; return $? ;;
  esac
}

select_nvidia_vulkan_icd() {
  # NVIDIA Container Toolkit normally injects the host driver's ICD into one of
  # these locations. Only pin VK_ICD_FILENAMES when the file actually exists.
  local icd
  for icd in \
    /usr/share/vulkan/icd.d/nvidia_icd.json \
    /etc/vulkan/icd.d/nvidia_icd.json \
    /usr/share/vulkan/icd.d/nvidia_icd.x86_64.json \
    /etc/vulkan/icd.d/nvidia_icd.x86_64.json; do
    if [[ -f "$icd" ]]; then
      export VK_ICD_FILENAMES="${VK_ICD_FILENAMES:-$icd}"
      return 0
    fi
  done
  return 1
}

if gpu_requested; then
  export LIBGL_ALWAYS_SOFTWARE=0
  export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
  export __GLX_VENDOR_LIBRARY_NAME="${__GLX_VENDOR_LIBRARY_NAME:-nvidia}"
  select_nvidia_vulkan_icd || true
else
  export LIBGL_ALWAYS_SOFTWARE=1
  export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
fi

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
exit 0
SH
    chmod +x /usr/bin/fbsetbg || true
  fi

  # If anything still tries to open xmessage, make it harmless too.
  if ! command -v xmessage >/dev/null 2>&1; then
    cat > /usr/local/bin/xmessage <<'SH'
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

start_virtual_display() {
  if gpu_requested && command -v Xorg >/dev/null 2>&1; then
    local xorg_conf="${LOG_DIR}/xorg-nvidia.conf"

    log "GPU display requested; trying NVIDIA Xorg on ${DISPLAY}"
    cat > "$xorg_conf" <<EOF2
Section "ServerLayout"
    Identifier "glitch-layout"
    Screen 0 "glitch-screen"
EndSection

Section "Device"
    Identifier "glitch-nvidia"
    Driver "nvidia"
    Option "AllowEmptyInitialConfiguration" "true"
    Option "UseDisplayDevice" "None"
    Option "VirtualHeads" "1"
EndSection

Section "Monitor"
    Identifier "glitch-monitor"
    HorizSync 28.0-80.0
    VertRefresh 48.0-75.0
EndSection

Section "Screen"
    Identifier "glitch-screen"
    Device "glitch-nvidia"
    Monitor "glitch-monitor"
    DefaultDepth ${DEPTH}
    SubSection "Display"
        Depth ${DEPTH}
        Virtual ${WIDTH} ${HEIGHT}
    EndSubSection
EndSection
EOF2

    Xorg "$DISPLAY" \
      -config "$xorg_conf" \
      -noreset \
      -nolisten tcp \
      +extension GLX \
      +extension RANDR \
      +extension RENDER \
      -logfile "$LOG_DIR/xorg.log" \
      > "$LOG_DIR/xorg.stdout.log" 2>&1 &
    XVFB_PID=$!
    sleep 2

    if kill -0 "$XVFB_PID" 2>/dev/null; then
      log "NVIDIA Xorg started with PID ${XVFB_PID}"
      return 0
    fi

    warn "NVIDIA Xorg failed; falling back to Xvfb. See ${LOG_DIR}/xorg.log"
    tail -n 120 "$LOG_DIR/xorg.log" || true
  fi

  log "Starting virtual X display with Xvfb"
  Xvfb "$DISPLAY" -screen 0 "${WIDTH}x${HEIGHT}x${DEPTH}" -nolisten tcp \
    > "$LOG_DIR/xvfb.log" 2>&1 &
  XVFB_PID=$!
  sleep 1
  check_process_alive "$XVFB_PID" "X display"
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


url_decode_component() {
  python3 - "$1" <<'PY'
import sys
import urllib.parse
print(urllib.parse.unquote_plus(sys.argv[1]))
PY
}

extract_install_id_from_websockify_log() {
  python3 - "$LOG_DIR/websockify.log" <<'PY'
import pathlib
import re
import sys
import urllib.parse

path = pathlib.Path(sys.argv[1])
try:
    text = path.read_text(errors='ignore')[-250000:]
except Exception:
    sys.exit(1)

keys = [
    'install_id',
    'installId',
    'glitch_install_id',
    'user_install_id',
    'session_id',
    'sessionId',
]

# websockify static logs usually include the full request path, including query string.
# Accept either raw URL query strings or escaped/encoded variants in logs.
for key in keys:
    pattern = re.compile(r'(?:[?&]|%3F|%26|\\?)' + re.escape(key) + r'=([^\s&"\'<>]+)', re.IGNORECASE)
    for match in pattern.finditer(text):
        value = urllib.parse.unquote_plus(match.group(1)).strip()
        value = value.strip(' ,;\'"')
        if value and not value.startswith('YOUR_') and value.lower() not in {'undefined', 'null', 'none'}:
            print(value)
            sys.exit(0)

# Fallback: allow bare key=value anywhere in the recent log tail.
for key in keys:
    pattern = re.compile(re.escape(key) + r'=([^\s&"\'<>]+)', re.IGNORECASE)
    for match in pattern.finditer(text):
        value = urllib.parse.unquote_plus(match.group(1)).strip()
        value = value.strip(' ,;\'"')
        if value and not value.startswith('YOUR_') and value.lower() not in {'undefined', 'null', 'none'}:
            print(value)
            sys.exit(0)

sys.exit(1)
PY
}


# GLITCH_LOADING_SPLASH_V3: visible loading state instead of a black X desktop.
GLITCH_SPLASH_PID="${GLITCH_SPLASH_PID:-}"
show_glitch_loading_splash() {
  local message="${1:-Glitch is preparing Veloren...}"
  if [ -z "${DISPLAY:-}" ]; then return 0; fi
  if command -v xsetroot >/dev/null 2>&1; then
    xsetroot -solid "#10131a" >/dev/null 2>&1 || true
  fi
  if command -v xmessage >/dev/null 2>&1; then
    pkill -f "xmessage.*Glitch" >/dev/null 2>&1 || true
    (xmessage -center -buttons "" -geometry 780x190 "$message" >/dev/null 2>&1 || true) &
    GLITCH_SPLASH_PID="$!"
  fi
}

hide_glitch_loading_splash() {
  if [ -n "${GLITCH_SPLASH_PID:-}" ]; then
    kill "$GLITCH_SPLASH_PID" >/dev/null 2>&1 || true
    GLITCH_SPLASH_PID=""
  fi
  pkill -f "xmessage.*Glitch" >/dev/null 2>&1 || true
}

# GLITCH_CLOUD_SAVE_V1: archive/restore streamed Veloren userdata through Glitch saves.
glitch_api_base() {
  printf "%s" "${GLITCH_API_BASE_URL:-https://api.glitch.fun/api}" | sed 's#/*$##'
}

glitch_cloud_save_url() {
  printf "%s/titles/%s/installs/%s/saves" "$(glitch_api_base)" "${GLITCH_TITLE_ID}" "${GLITCH_INSTALL_ID}"
}

glitch_restore_cloud_save() {
  if [ "${GLITCH_CLOUD_SAVE_ENABLED:-1}" != "1" ]; then
    log "Glitch cloud save restore disabled."
    return 0
  fi
  if [ -z "${GLITCH_TITLE_ID:-}" ] || [ -z "${GLITCH_TITLE_TOKEN:-}" ] || [ -z "${GLITCH_INSTALL_ID:-}" ]; then
    log "Glitch cloud save restore skipped: missing title id/token/install id."
    return 0
  fi

  local url tmp_json tmp_payload tmp_archive version
  url="$(glitch_cloud_save_url)?include_payload=1"
  tmp_json="/tmp/glitch-cloud-save-list.json"
  tmp_payload="/tmp/glitch-cloud-save-payload.b64"
  tmp_archive="/tmp/glitch-cloud-save-restore.tar.gz"

  log "Checking Glitch cloud save slot ${GLITCH_CLOUD_SAVE_SLOT:-0}."
  if ! curl -fsS \
      -H "Accept: application/json" \
      -H "Authorization: Bearer ${GLITCH_TITLE_TOKEN}" \
      "$url" -o "$tmp_json"; then
    log "No Glitch cloud save restored: list request failed or no access."
    return 0
  fi

  if ! python3 - "$tmp_json" "$tmp_payload" "${GLITCH_CLOUD_SAVE_SLOT:-0}" /tmp/glitch-cloud-save-version <<'PY_SAVE_RESTORE'
import json, sys
src, payload_out, slot, version_out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
data = json.load(open(src, 'r', encoding='utf-8'))
items = data.get('data', data if isinstance(data, list) else [])
chosen = None
for item in items:
    try:
        if int(item.get('slot_index', -1)) == slot:
            chosen = item
            break
    except Exception:
        pass
if not chosen or not chosen.get('payload'):
    sys.exit(2)
open(payload_out, 'w', encoding='utf-8').write(chosen['payload'])
open(version_out, 'w', encoding='utf-8').write(str(chosen.get('version') or 0))
PY_SAVE_RESTORE
  then
    log "No Glitch cloud save payload found for slot ${GLITCH_CLOUD_SAVE_SLOT:-0}."
    return 0
  fi

  if base64 -d "$tmp_payload" > "$tmp_archive" 2>/dev/null || base64 --decode "$tmp_payload" > "$tmp_archive" 2>/dev/null; then
    mkdir -p /opt/userdata
    if tar -xzf "$tmp_archive" -C /opt/userdata; then
      version="$(cat /tmp/glitch-cloud-save-version 2>/dev/null || echo 0)"
      log "Restored Glitch cloud save slot ${GLITCH_CLOUD_SAVE_SLOT:-0}, version ${version}."
      return 0
    fi
  fi

  log "Glitch cloud save payload existed but could not be decoded/extracted. Starting fresh."
  return 0
}

glitch_upload_cloud_save() {
  if [ "${GLITCH_CLOUD_SAVE_ENABLED:-1}" != "1" ]; then return 0; fi
  if [ -z "${GLITCH_TITLE_ID:-}" ] || [ -z "${GLITCH_TITLE_TOKEN:-}" ] || [ -z "${GLITCH_INSTALL_ID:-}" ]; then return 0; fi
  if [ ! -d /opt/userdata ]; then return 0; fi

  local archive payload json checksum bytes base_version url now
  archive="/tmp/glitch-cloud-save-upload.tar.gz"
  payload="/tmp/glitch-cloud-save-upload.b64"
  json="/tmp/glitch-cloud-save-upload.json"
  url="$(glitch_cloud_save_url)"
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  base_version="$(cat /tmp/glitch-cloud-save-version 2>/dev/null || echo 0)"

  tar -czf "$archive" -C /opt/userdata server-runtime client-runtime 2>/dev/null || return 0
  bytes="$(wc -c < "$archive" | tr -d ' ')"
  if [ "$bytes" -gt "${GLITCH_CLOUD_SAVE_MAX_BYTES:-9500000}" ]; then
    log "Glitch cloud save skipped: archive ${bytes} bytes exceeds max ${GLITCH_CLOUD_SAVE_MAX_BYTES:-9500000}."
    return 0
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    checksum="$(sha256sum "$archive" | awk '{print $1}')"
  else
    checksum="$(shasum -a 256 "$archive" | awk '{print $1}')"
  fi
  if base64 -w 0 "$archive" > "$payload" 2>/dev/null; then :; else base64 "$archive" | tr -d '\n' > "$payload"; fi

  python3 - "$payload" "$json" "$checksum" "$base_version" "$now" "$bytes" "${GLITCH_CLOUD_SAVE_SLOT:-0}" <<'PY_SAVE_UPLOAD'
import json, sys
payload_path, json_path, checksum, base_version, now, size_bytes, slot = sys.argv[1:]
payload = open(payload_path, 'r', encoding='utf-8').read().strip()
try:
    base_version = int(base_version or 0)
except Exception:
    base_version = 0
body = {
    'slot_index': int(slot),
    'slot_name': 'Veloren streamed native autosave',
    'payload': payload,
    'checksum': checksum,
    'base_version': base_version,
    'save_type': 'auto',
    'client_timestamp': now,
    'metadata': {
        'source': 'veloren_streamed_native',
        'archive': 'server-runtime+client-runtime',
        'size_bytes': int(size_bytes),
    },
    'platform': 'streamed_native',
    'game_version': 'veloren-glitch',
    'last_played_at': now,
}
open(json_path, 'w', encoding='utf-8').write(json.dumps(body))
PY_SAVE_UPLOAD

  local response status
  response="/tmp/glitch-cloud-save-response.json"
  status="$(curl -sS -o "$response" -w '%{http_code}' \
      -X POST \
      -H "Accept: application/json" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer ${GLITCH_TITLE_TOKEN}" \
      --data-binary "@$json" \
      "$url" || echo 000)"

  if [ "$status" = "200" ] || [ "$status" = "201" ]; then
    python3 - "$response" /tmp/glitch-cloud-save-version <<'PY_SAVE_VERSION' || true
import json, sys
try:
    data = json.load(open(sys.argv[1], 'r', encoding='utf-8'))
    item = data.get('data', data)
    version = item.get('version') or item.get('save', {}).get('version') or 0
    open(sys.argv[2], 'w', encoding='utf-8').write(str(version))
except Exception:
    pass
PY_SAVE_VERSION
    log "Uploaded Glitch cloud save slot ${GLITCH_CLOUD_SAVE_SLOT:-0} (${bytes} bytes)."
  else
    log "Glitch cloud save upload returned HTTP ${status}; keeping local runtime data for this session."
    tail -c 600 "$response" 2>/dev/null || true
  fi
}

glitch_cloud_save_loop() {
  while true; do
    sleep "${GLITCH_CLOUD_SAVE_INTERVAL_SECONDS:-60}" || true
    glitch_upload_cloud_save || true
  done
}

start_glitch_cloud_save_loop() {
  if [ "${GLITCH_CLOUD_SAVE_ENABLED:-1}" = "1" ]; then
    (glitch_cloud_save_loop) &
    GLITCH_CLOUD_SAVE_LOOP_PID="$!"
    log "Started Glitch cloud-save loop every ${GLITCH_CLOUD_SAVE_INTERVAL_SECONDS:-60}s."
  fi
}

glitch_shutdown_save() {
  glitch_upload_cloud_save || true
}



# GLITCH_CONTAINER_AUDIO_PROXY_HELPERS_V1:
# Route one public container port to noVNC and audio internally. This keeps the
# VM/matchmaker proxy unchanged and scopes audio behavior to this image only.
start_glitch_container_public_proxy() {
  local public_port="${GLITCH_PUBLIC_PORT:-6080}"
  local novnc_port="${GLITCH_NOVNC_INTERNAL_PORT:-6082}"
  local audio_port="${GLITCH_AUDIO_PORT:-6081}"

  if ! command -v nginx >/dev/null 2>&1; then
    log "Container proxy: nginx missing; cannot expose audio on same public port."
    return 0
  fi

  mkdir -p /tmp/veloren-web /tmp/glitch-nginx-client-body /tmp/glitch-nginx-proxy

  cat >/tmp/glitch-nginx.conf <<EOF_NGINX
worker_processes  1;
error_log /tmp/veloren-web/glitch-nginx-error.log warn;
pid /tmp/glitch-nginx.pid;

events {
    worker_connections 1024;
}

http {
    access_log /tmp/veloren-web/glitch-nginx-access.log;
    client_body_temp_path /tmp/glitch-nginx-client-body;
    proxy_temp_path /tmp/glitch-nginx-proxy;
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    map \$http_upgrade \$connection_upgrade {
        default upgrade;
        '' close;
    }

    server {
        listen 0.0.0.0:${public_port};
        server_name _;

        proxy_http_version 1.1;
        proxy_buffering off;
        proxy_request_buffering off;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;

        location = /glitch-audio-status {
            proxy_pass http://127.0.0.1:${audio_port};
            proxy_set_header Host \$host;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        }

        location /glitch-audio {
            proxy_pass http://127.0.0.1:${audio_port};
            proxy_set_header Host \$host;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        }

        location /audio {
            proxy_pass http://127.0.0.1:${audio_port};
            proxy_set_header Host \$host;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        }

        location / {
            proxy_pass http://127.0.0.1:${novnc_port};
            proxy_set_header Host \$host;
            proxy_set_header Upgrade \$http_upgrade;
            proxy_set_header Connection \$connection_upgrade;
            proxy_set_header X-Forwarded-Proto \$scheme;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        }
    }
}
EOF_NGINX

  if pgrep -f "nginx.*glitch-nginx.conf" >/dev/null 2>&1; then
    log "Container proxy: nginx already running on :${public_port}"
    return 0
  fi

  log "Container proxy: starting nginx on :${public_port} -> noVNC :${novnc_port}, audio :${audio_port}"
  nginx -c /tmp/glitch-nginx.conf -g 'daemon off;' > /tmp/veloren-web/glitch-nginx.log 2>&1 &
  export GLITCH_CONTAINER_PROXY_PID="$!"

  sleep 1
}







# GLITCH_USER_PULSEAUDIO_HELPERS_V1:
ensure_glitch_runtime_user() {
  if ! id -u "${GLITCH_RUNTIME_USER:-glitch}" >/dev/null 2>&1; then
    groupadd -r "${GLITCH_RUNTIME_USER:-glitch}" 2>/dev/null || true
    useradd -r -g "${GLITCH_RUNTIME_USER:-glitch}" -m -d "${GLITCH_RUNTIME_HOME:-/home/glitch}" -s /bin/bash "${GLITCH_RUNTIME_USER:-glitch}" 2>/dev/null || true
  fi

  mkdir -p \
    "${GLITCH_RUNTIME_HOME:-/home/glitch}" \
    "${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}" \
    "${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}" \
    /tmp/veloren-web \
    /opt/userdata/client-runtime \
    /opt/userdata/client-runtime/voxygen

  chown -R "${GLITCH_RUNTIME_USER:-glitch}:${GLITCH_RUNTIME_USER:-glitch}" \
    "${GLITCH_RUNTIME_HOME:-/home/glitch}" \
    "${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}" \
    "${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}" \
    /tmp/veloren-web \
    /opt/userdata/client-runtime 2>/dev/null || true

  chmod 700 "${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}" "${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}" 2>/dev/null || true
  chmod 775 /tmp/veloren-web 2>/dev/null || true
}

run_as_glitch() {
  ensure_glitch_runtime_user

  if command -v gosu >/dev/null 2>&1; then
    gosu "${GLITCH_RUNTIME_USER:-glitch}" \
      env \
        HOME="${GLITCH_RUNTIME_HOME:-/home/glitch}" \
        USER="${GLITCH_RUNTIME_USER:-glitch}" \
        LOGNAME="${GLITCH_RUNTIME_USER:-glitch}" \
        XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}" \
        PULSE_RUNTIME_PATH="${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}" \
        PULSE_SERVER="${PULSE_SERVER:-unix:${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}/native}" \
        PULSE_SINK="${PULSE_SINK:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}}" \
        PULSE_SOURCE="${PULSE_SOURCE:-${GLITCH_AUDIO_SOURCE:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}.monitor}}" \
        SDL_AUDIODRIVER="${SDL_AUDIODRIVER:-pulse}" \
        ALSOFT_DRIVERS="${ALSOFT_DRIVERS:-pulse}" \
        "$@"
  elif command -v runuser >/dev/null 2>&1; then
    runuser -u "${GLITCH_RUNTIME_USER:-glitch}" -- \
      env \
        HOME="${GLITCH_RUNTIME_HOME:-/home/glitch}" \
        USER="${GLITCH_RUNTIME_USER:-glitch}" \
        LOGNAME="${GLITCH_RUNTIME_USER:-glitch}" \
        XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}" \
        PULSE_RUNTIME_PATH="${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}" \
        PULSE_SERVER="${PULSE_SERVER:-unix:${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}/native}" \
        PULSE_SINK="${PULSE_SINK:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}}" \
        PULSE_SOURCE="${PULSE_SOURCE:-${GLITCH_AUDIO_SOURCE:-${GLITCH_AUDIO_SINK:-glitch_stream_sink}.monitor}}" \
        SDL_AUDIODRIVER="${SDL_AUDIODRIVER:-pulse}" \
        ALSOFT_DRIVERS="${ALSOFT_DRIVERS:-pulse}" \
        "$@"
  else
    echo "FAIL: neither gosu nor runuser exists for non-root audio runtime" >&2
    return 1
  fi
}

start_glitch_pulseaudio_daemon() {
  ensure_glitch_runtime_user

  if run_as_glitch pactl info >/tmp/veloren-web/pactl-info-preexisting.log 2>&1; then
    log "Audio: PulseAudio user daemon already reachable."
    return 0
  fi

  rm -f "${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}/native" "${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}/pid" 2>/dev/null || true

  cat >/tmp/glitch-pulseaudio.pa <<EOF_GLITCH_USER_PULSE
.nofail
load-module module-native-protocol-unix auth-anonymous=1 socket=${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}/native
load-module module-null-sink sink_name=${GLITCH_AUDIO_SINK:-glitch_stream_sink} sink_properties=device.description=Glitch_Stream_Audio rate=${GLITCH_AUDIO_SAMPLE_RATE:-48000} channels=${GLITCH_AUDIO_CHANNELS:-2}
load-module module-always-sink
.fail
set-default-sink ${GLITCH_AUDIO_SINK:-glitch_stream_sink}
set-default-source ${GLITCH_AUDIO_SINK:-glitch_stream_sink}.monitor
EOF_GLITCH_USER_PULSE

  chown "${GLITCH_RUNTIME_USER:-glitch}:${GLITCH_RUNTIME_USER:-glitch}" /tmp/glitch-pulseaudio.pa 2>/dev/null || true

  log "Audio: starting per-user PulseAudio daemon as ${GLITCH_RUNTIME_USER:-glitch} at ${PULSE_SERVER}"
  run_as_glitch pulseaudio \
    --daemonize=yes \
    --exit-idle-time=-1 \
    --disallow-exit \
    --log-target=file:/tmp/veloren-web/pulseaudio-start.log \
    -nF /tmp/glitch-pulseaudio.pa || true

  for i in $(seq 1 30); do
    if run_as_glitch pactl info >/tmp/veloren-web/pactl-info-after-start.log 2>&1; then
      log "Audio: PulseAudio user daemon is ready."
      return 0
    fi
    sleep 0.25
  done

  log "Audio: PulseAudio user daemon did not become ready. Startup log follows:"
  cat /tmp/veloren-web/pulseaudio-start.log 2>/dev/null || true
  return 1
}


# GLITCH_AUDIO_HELPERS_V1: PulseAudio capture + same-origin browser audio player.
find_novnc_html_for_audio_patch() {
  for candidate in \
    /usr/share/novnc/vnc.html \
    /usr/share/novnc/index.html \
    /usr/share/noVNC/vnc.html \
    /usr/share/noVNC/index.html \
    /opt/noVNC/vnc.html \
    /opt/noVNC/index.html \
    /opt/novnc/vnc.html \
    /opt/novnc/index.html \
    /usr/local/share/novnc/vnc.html \
    /usr/local/share/novnc/index.html; do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  find /usr/share /opt /usr/local/share -maxdepth 4 -type f \( -name 'vnc.html' -o -name 'index.html' \) 2>/dev/null | head -n1
}

inject_glitch_audio_player_into_novnc() {
  if [ "${GLITCH_AUDIO_ENABLED:-1}" != "1" ]; then
    return 0
  fi

  local html
  html="$(find_novnc_html_for_audio_patch || true)"

  if [ -z "$html" ] || [ ! -f "$html" ]; then
    log "Audio: noVNC HTML file not found; browser audio element will not be injected."
    return 0
  fi

  if grep -q "GLITCH_STREAMED_NATIVE_AUDIO_PLAYER_V1" "$html"; then
    log "Audio: noVNC audio player already injected into $html"
    return 0
  fi

  log "Audio: injecting browser audio player into $html"

  python3 - "$html" <<'GLITCH_AUDIO_INJECT_PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
text = path.read_text(errors="ignore")

script = r"""
<script id="glitch-streamed-native-audio-player">
/* GLITCH_STREAMED_NATIVE_AUDIO_PLAYER_V1 */
(function () {
  if (window.__glitchStreamedNativeAudioV1) return;
  window.__glitchStreamedNativeAudioV1 = true;

  function log() {
    try {
      console.log.apply(console, ['[glitch-audio]'].concat(Array.from(arguments)));
    } catch (e) {}
  }

  function ensureButton(audio) {
    if (document.getElementById('glitch-audio-unlock-button')) return;

    var button = document.createElement('button');
    button.id = 'glitch-audio-unlock-button';
    button.textContent = 'Enable game audio';
    button.style.position = 'fixed';
    button.style.right = '14px';
    button.style.bottom = '14px';
    button.style.zIndex = '2147483647';
    button.style.padding = '10px 14px';
    button.style.borderRadius = '12px';
    button.style.border = '1px solid rgba(255,255,255,0.22)';
    button.style.background = 'rgba(12, 18, 32, 0.88)';
    button.style.color = '#fff';
    button.style.fontFamily = 'system-ui, -apple-system, Segoe UI, sans-serif';
    button.style.fontSize = '14px';
    button.style.cursor = 'pointer';
    button.style.boxShadow = '0 10px 30px rgba(0,0,0,.35)';

    button.addEventListener('click', function () {
      tryPlay(audio);
    });

    document.documentElement.appendChild(button);
  }

  function removeButton() {
    var button = document.getElementById('glitch-audio-unlock-button');
    if (button && button.parentNode) button.parentNode.removeChild(button);
  }

  function tryPlay(audio) {
    if (!audio) return;

    if (!audio.src) {
      audio.src = '/glitch-audio.webm?t=' + Date.now();
    }

    audio.muted = false;
    audio.volume = 1.0;
    audio.setAttribute('playsinline', 'true');
    audio.setAttribute('webkit-playsinline', 'true');

    var p = audio.play();

    if (p && typeof p.then === 'function') {
      p.then(function () {
        removeButton();
        log('audio playback started');
      }).catch(function (err) {
        log('audio playback blocked/waiting for user gesture', err && err.message ? err.message : err);
        ensureButton(audio);
      });
    }
  }

  function boot() {
    var audio = document.getElementById('glitch-streamed-native-audio');
    if (!audio) {
      audio = document.createElement('audio');
      audio.id = 'glitch-streamed-native-audio';
      audio.preload = 'auto';
      audio.autoplay = true;
      audio.controls = false;
      audio.style.display = 'none';
      audio.crossOrigin = 'anonymous';
      document.documentElement.appendChild(audio);
    }

    tryPlay(audio);

    ['pointerdown', 'mousedown', 'touchstart', 'keydown', 'click'].forEach(function (eventName) {
      window.addEventListener(eventName, function () {
        tryPlay(audio);
      }, { capture: true, passive: true });
      document.addEventListener(eventName, function () {
        tryPlay(audio);
      }, { capture: true, passive: true });
    });

    audio.addEventListener('error', function () {
      log('audio element error; reconnecting shortly');
      setTimeout(function () {
        audio.src = '/glitch-audio.webm?t=' + Date.now();
        tryPlay(audio);
      }, 1200);
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot, { once: true });
  } else {
    boot();
  }
})();
</script>
"""

if "GLITCH_STREAMED_NATIVE_AUDIO_PLAYER_V1" in text:
    raise SystemExit(0)

lower = text.lower()
idx = lower.rfind("</body>")
if idx >= 0:
    text = text[:idx] + script + "\n" + text[idx:]
else:
    text = text + "\n" + script + "\n"

path.write_text(text)
GLITCH_AUDIO_INJECT_PY
}

start_glitch_audio_stack() {
  if [ "${GLITCH_AUDIO_ENABLED:-1}" != "1" ]; then
    log "Audio: disabled by GLITCH_AUDIO_ENABLED=${GLITCH_AUDIO_ENABLED:-}"
    return 0
  fi

  if ! command -v pulseaudio >/dev/null 2>&1; then
    log "Audio: pulseaudio missing; audio streaming disabled."
    return 0
  fi

  if ! command -v ffmpeg >/dev/null 2>&1; then
    log "Audio: ffmpeg missing; audio streaming disabled."
    return 0
  fi

  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/glitch-xdg-runtime}"
  export PULSE_RUNTIME_PATH="${PULSE_RUNTIME_PATH:-/tmp/glitch-pulse}"
  export HOME="${HOME:-/tmp/veloren-home}"
  mkdir -p "$XDG_RUNTIME_DIR" "$PULSE_RUNTIME_PATH" "$HOME" /tmp/veloren-web
  chmod 700 "$XDG_RUNTIME_DIR" "$PULSE_RUNTIME_PATH" "$HOME" || true

  cat >/tmp/glitch-asound.conf <<'EOF_ASOUND'
pcm.!default {
    type pulse
}
ctl.!default {
    type pulse
}
EOF_ASOUND

  export ALSA_CONFIG_PATH=/tmp/glitch-asound.conf
  export SDL_AUDIODRIVER="${SDL_AUDIODRIVER:-pulse}"
  export ALSOFT_DRIVERS="${ALSOFT_DRIVERS:-pulse}"
  export PULSE_PROP="media.role=game"

  if ! pulseaudio --check >/dev/null 2>&1; then
    log "Audio: starting PulseAudio daemon"
    start_glitch_pulseaudio_daemon || true
    sleep 1
  fi

  start_glitch_pulseaudio_daemon || true

  if ! run_as_glitch pactl info >/tmp/veloren-web/pactl-info.log 2>&1; then
    log "Audio: pactl cannot talk to PulseAudio; audio streaming disabled."
    cat /tmp/veloren-web/pactl-info.log || true
    return 0
  fi

  local sink="${GLITCH_AUDIO_SINK:-glitch_stream_sink}"

  if ! run_as_glitch pactl list short sinks 2>/dev/null | awk '{print $2}' | grep -qx "$sink"; then
    log "Audio: creating PulseAudio null sink $sink"
    run_as_glitch pactl load-module module-null-sink \
      "sink_name=$sink" \
      "sink_properties=device.description=Glitch_Stream_Audio" \
      "rate=${GLITCH_AUDIO_SAMPLE_RATE:-48000}" \
      "channels=${GLITCH_AUDIO_CHANNELS:-2}" >/tmp/veloren-web/pactl-load-null-sink.log 2>&1 || true
  fi

  run_as_glitch pactl set-default-sink "$sink" >/tmp/veloren-web/pactl-default-sink.log 2>&1 || true
  export GLITCH_AUDIO_SOURCE="${GLITCH_AUDIO_SOURCE:-${sink}.monitor}"

  log "Audio: default sink=$sink source=$GLITCH_AUDIO_SOURCE port=${GLITCH_AUDIO_PORT:-6081}"

  cat >/tmp/glitch-audio-server.py <<'GLITCH_AUDIO_SERVER_PY'
import json
import os
import signal
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(os.environ.get("GLITCH_AUDIO_PORT", "6081"))
SOURCE = os.environ.get("GLITCH_AUDIO_SOURCE", "glitch_stream_sink.monitor")
BITRATE = os.environ.get("GLITCH_AUDIO_BITRATE", "96000")
RATE = os.environ.get("GLITCH_AUDIO_SAMPLE_RATE", "48000")
CHANNELS = os.environ.get("GLITCH_AUDIO_CHANNELS", "2")

class Handler(BaseHTTPRequestHandler):
    server_version = "GlitchAudio/1.0"

    def log_message(self, fmt, *args):
        sys.stderr.write("[glitch-audio] " + (fmt % args) + "\n")
        sys.stderr.flush()

    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Range")

    def do_OPTIONS(self):
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_GET(self):
        if self.path.startswith("/glitch-audio-status"):
            payload = {
                "ok": True,
                "source": SOURCE,
                "bitrate": BITRATE,
                "sample_rate": RATE,
                "channels": CHANNELS,
            }
            body = json.dumps(payload).encode("utf-8")
            self.send_response(200)
            self._cors()
            self.send_header("Content-Type", "application/json")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        if not (self.path.startswith("/glitch-audio") or self.path.startswith("/audio")):
            self.send_response(404)
            self._cors()
            self.end_headers()
            return

        cmd = [
            "ffmpeg",
            "-hide_banner",
            "-loglevel", "warning",
            "-nostdin",
            "-f", "pulse",
            "-i", SOURCE,
            "-vn",
            "-ac", CHANNELS,
            "-ar", RATE,
            "-c:a", "libopus",
            "-b:a", BITRATE,
            "-application", "lowdelay",
            "-f", "webm",
            "pipe:1",
        ]

        self.log_message("starting stream source=%s", SOURCE)
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)

        self.send_response(200)
        self._cors()
        self.send_header("Content-Type", "audio/webm; codecs=opus")
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()

        try:
            while True:
                chunk = proc.stdout.read(16384)
                if not chunk:
                    break
                self.wfile.write(chunk)
                self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass
        finally:
            try:
                proc.send_signal(signal.SIGTERM)
                proc.wait(timeout=2)
            except Exception:
                try:
                    proc.kill()
                except Exception:
                    pass

            try:
                err = proc.stderr.read().decode("utf-8", "replace").strip()
                if err:
                    self.log_message("ffmpeg: %s", err[-2000:])
            except Exception:
                pass

def main():
    httpd = ThreadingHTTPServer(("0.0.0.0", PORT), Handler)
    print(f"[glitch-audio] listening on 0.0.0.0:{PORT}, source={SOURCE}", flush=True)
    httpd.serve_forever()

if __name__ == "__main__":
    main()
GLITCH_AUDIO_SERVER_PY

  chmod +x /tmp/glitch-audio-server.py

  if ! pgrep -f "glitch-audio-server.py" >/dev/null 2>&1; then
    log "Audio: starting HTTP audio streamer on port ${GLITCH_AUDIO_PORT:-6081}"
    run_as_glitch python3 /tmp/glitch-audio-server.py > /tmp/veloren-web/glitch-audio-server.log 2>&1 &
    export GLITCH_AUDIO_SERVER_PID="$!"
  fi

  inject_glitch_audio_player_into_novnc
}


wait_for_glitch_install_id_from_iframe() {
  if [[ "$WEB_MODE" == "server_only" ]]; then
    return 0
  fi

  if [[ -n "${GLITCH_INSTALL_ID:-}" ]]; then
    log "Using GLITCH_INSTALL_ID supplied in environment: ${GLITCH_INSTALL_ID}"
    export GLITCH_USER_INSTALL_ID="${GLITCH_USER_INSTALL_ID:-$GLITCH_INSTALL_ID}"
    export GLITCH_SESSION_ID="${GLITCH_SESSION_ID:-$GLITCH_INSTALL_ID}"
    return 0
  fi

  local launch_file="${VELOREN_LAUNCH_ENV_FILE:-${LOG_DIR}/glitch-launch.env}"
  local timeout_s="${VELOREN_INSTALL_WAIT_TIMEOUT:-0}"
  local start now found
  start="$(date +%s)"

  log "No GLITCH_INSTALL_ID at boot. Idle launcher is waiting for iframe query install_id/session_id."
  log "Expected URL example: /vnc.html?autoconnect=1&install_id=<real-install-id>"
  log "Install wait timeout: ${timeout_s}s (0 means wait forever)."

  while true; do
    if [[ -f "$launch_file" ]]; then
      # shellcheck disable=SC1090
      source "$launch_file" || true
      if [[ -n "${GLITCH_INSTALL_ID:-}" ]]; then
        log "Received GLITCH_INSTALL_ID from ${launch_file}: ${GLITCH_INSTALL_ID}"
        export GLITCH_USER_INSTALL_ID="${GLITCH_USER_INSTALL_ID:-$GLITCH_INSTALL_ID}"
        export GLITCH_SESSION_ID="${GLITCH_SESSION_ID:-$GLITCH_INSTALL_ID}"
        return 0
      fi
    fi

    found="$(extract_install_id_from_websockify_log || true)"
    if [[ -n "$found" ]]; then
      export GLITCH_INSTALL_ID="$found"
      export GLITCH_USER_INSTALL_ID="${GLITCH_USER_INSTALL_ID:-$found}"
      export GLITCH_SESSION_ID="${GLITCH_SESSION_ID:-$found}"
      cat > "$launch_file" <<EOF_LAUNCH
GLITCH_INSTALL_ID=${GLITCH_INSTALL_ID}
GLITCH_USER_INSTALL_ID=${GLITCH_USER_INSTALL_ID}
GLITCH_SESSION_ID=${GLITCH_SESSION_ID}
EOF_LAUNCH
      log "Received install id from iframe/noVNC request log: ${GLITCH_INSTALL_ID}"
      return 0
    fi

    if [[ "$timeout_s" != "0" ]]; then
      now="$(date +%s)"
      if (( now - start > timeout_s )); then
        fail "Timed out waiting for iframe install_id/session_id after ${timeout_s}s."
      fi
    fi

    sleep 1
  done
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
log "Renderer mode: GPU_MODE=${GPU_MODE}, LIBGL_ALWAYS_SOFTWARE=${LIBGL_ALWAYS_SOFTWARE:-unset}, WGPU_BACKEND=${WGPU_BACKEND:-unset}, VK_ICD_FILENAMES=${VK_ICD_FILENAMES:-auto}"
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

if [[ "$GLITCH_TITLE_TOKEN" == YOUR_* || "${GLITCH_INSTALL_ID:-}" == YOUR_* ]]; then
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

start_virtual_display

disable_fluxbox_wallpaper_popup

log "Starting window manager"
create_fluxbox_config
fluxbox -rc "$FLUXBOX_INIT" > "$LOG_DIR/fluxbox.log" 2>&1 &
FLUXBOX_PID=$!
sleep 1
check_process_alive "$FLUXBOX_PID" "fluxbox"

log "Starting PulseAudio if available"
start_glitch_pulseaudio_daemon || true

start_x11vnc_robust

log "Starting noVNC/websockify on 0.0.0.0:${WEB_PORT}"
websockify --web=/usr/share/novnc/ "0.0.0.0:${WEB_PORT}" "127.0.0.1:${VNC_PORT}" \
  > "$LOG_DIR/websockify.log" 2>&1 &
WEBSOCKIFY_PID=$!
wait_for_http "http://127.0.0.1:${WEB_PORT}/vnc.html" "$STREAM_READY_TIMEOUT"
check_process_alive "$WEBSOCKIFY_PID" "websockify"
log "Stream endpoint is up: ${BROWSER_URL_PATH}"
start_glitch_container_public_proxy || true
start_glitch_audio_stack || true
show_glitch_loading_splash "Glitch is preparing Veloren...\nWaiting for your Glitch play session."

# Wait here, after noVNC is available, so the VM can sit idle until the
# Glitch iframe/session passes the real install ID in the iframe URL.

# GLITCH_SHUTDOWN_SAVE_TRAP_V1: upload cloud save on normal container shutdown.
trap 'glitch_shutdown_save || true' EXIT
wait_for_glitch_install_id_from_iframe
show_glitch_loading_splash "Starting Veloren...
Restoring your Glitch cloud save."
glitch_restore_cloud_save || true
start_glitch_cloud_save_loop || true
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

hide_glitch_loading_splash # GLITCH_HIDE_SPLASH_BEFORE_CLIENT
log "Starting Veloren Voxygen client"
log "Login username: ${GLITCH_INSTALL_ID:-<missing>}"
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
