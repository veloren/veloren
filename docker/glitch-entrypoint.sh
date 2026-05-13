#!/usr/bin/env bash
set -euo pipefail

: "${GLITCH_TITLE_ID:?GLITCH_TITLE_ID is required}"
: "${GLITCH_TITLE_TOKEN:?GLITCH_TITLE_TOKEN is required}"
: "${GLITCH_SHARED_PASSWORD:?GLITCH_SHARED_PASSWORD is required}"

VELOREN_USERDATA="${VELOREN_USERDATA:-/opt/userdata}"
SERVER_DIR="$VELOREN_USERDATA/server/server_config"
CLI_DIR="$VELOREN_USERDATA/server-cli"
mkdir -p "$SERVER_DIR" "$CLI_DIR" "$VELOREN_USERDATA/server/saves"

SERVER_NAME="${VELOREN_SERVER_NAME:-Veloren on Glitch}"
MAX_PLAYERS="${VELOREN_MAX_PLAYERS:-100}"
API_BASE="${GLITCH_API_BASE_URL:-https://api.glitch.fun/api}"
AUTH_MODE="${VELOREN_AUTH_MODE:-glitch}" # glitch | official | custom
AUTH_SERVER_URL="${VELOREN_AUTH_SERVER_URL:-https://auth.veloren.net}"
REWRITE_SETTINGS="${VELOREN_REWRITE_SERVER_SETTINGS:-1}"

case "$AUTH_MODE" in
  glitch)
    AUTH_SERVER_RON="Some(\"glitch://${API_BASE#https://}\")"
    ;;
  official|custom)
    AUTH_SERVER_RON="Some(\"${AUTH_SERVER_URL}\")"
    ;;
  none)
    AUTH_SERVER_RON="None"
    ;;
  *)
    echo "Unsupported VELOREN_AUTH_MODE='${AUTH_MODE}'. Use glitch, official, custom, or none." >&2
    exit 1
    ;;
esac

if [[ "$REWRITE_SETTINGS" == "1" || ! -s "$SERVER_DIR/settings.ron" ]]; then
  cat > "$SERVER_DIR/settings.ron" <<RON
(
    gameserver_protocols: [
        Tcp(address: "0.0.0.0:14004"),
        Tcp(address: "[::]:14004"),
    ],
    auth_server_address: ${AUTH_SERVER_RON},
    query_address: Some("0.0.0.0:14006"),
    max_players: ${MAX_PLAYERS},
    server_name: "${SERVER_NAME}",
)
RON
fi

if [[ "$REWRITE_SETTINGS" == "1" || ! -s "$CLI_DIR/settings.ron" ]]; then
  cat > "$CLI_DIR/settings.ron" <<RON
(
    web_address: "0.0.0.0:14005",
    web_chat_secret: None,
    ui_api_secret: None,
)
RON
fi

echo "=== Veloren Glitch Server ==="
echo "Title: ${GLITCH_TITLE_ID}"
echo "API: ${API_BASE}"
echo "Auth mode: ${AUTH_MODE}"
echo "Auth server: ${AUTH_SERVER_URL}"
echo "Max players: ${MAX_PLAYERS}"
echo "Userdata: ${VELOREN_USERDATA}"
echo "Server settings: $SERVER_DIR/settings.ron"

exec /opt/veloren-server-cli --non-interactive
