#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/glitch-deploy.sh [options]

Build/package helper for the Veloren Glitch streamed-native deployment.

Options:
  --build                  Build docker/Dockerfile.glitch-veloren-web locally.
  --image NAME             Local Docker image tag. Default: veloren-glitch-web:<version>
  --out-dir DIR            Directory for the upload ZIP. Default: ~/Downloads
  --version VERSION        Build/package version suffix. Default: UTC timestamp
  --zip-name NAME          Exact ZIP file name to write inside --out-dir.
  --test-install-id ID     Test install ID to reject if found in packageable files.
  --test-title-token TOK   Test title token to reject if found in packageable files.
  --skip-secret-scan       Skip test secret scanning.
  --upload-command CMD     Optional shell command to run after ZIP verification.
  -h, --help               Show this help.

Environment:
  TEST_INSTALL_ID          Same as --test-install-id.
  TEST_TITLE_TOKEN         Same as --test-title-token.
  GLITCH_DEPLOY_BUILD=1    Same as --build.
  GLITCH_UPLOAD_COMMAND    Same as --upload-command.

The upload command receives:
  GLITCH_ZIP               Absolute path to the verified upload ZIP.
  GLITCH_MANIFEST          Absolute path to glitch-streamed-native.json.
  GLITCH_DEPLOY_VERSION    Version used for this package.

Example:
  TEST_INSTALL_ID=... TEST_TITLE_TOKEN=... scripts/glitch-deploy.sh --build

  GLITCH_UPLOAD_COMMAND='glitch upload-build --manifest "$GLITCH_MANIFEST" "$GLITCH_ZIP"' \
    scripts/glitch-deploy.sh --version smoke-test
EOF
}

log() {
  printf '[glitch-deploy] %s\n' "$*"
}

fail() {
  printf '[glitch-deploy] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
cd "$repo_root"

version="${GLITCH_DEPLOY_VERSION:-$(date -u +%Y%m%d-%H%M%S)}"
out_dir="${GLITCH_DEPLOY_OUT_DIR:-${HOME}/Downloads}"
zip_name="${GLITCH_DEPLOY_ZIP_NAME:-veloren-glitch-streamed-native-${version}.zip}"
image="${GLITCH_DEPLOY_IMAGE:-veloren-glitch-web:${version}}"
build="${GLITCH_DEPLOY_BUILD:-0}"
skip_secret_scan=0
upload_command="${GLITCH_UPLOAD_COMMAND:-}"
test_install_id="${TEST_INSTALL_ID:-}"
test_title_token="${TEST_TITLE_TOKEN:-}"
zip_name_set=0
image_set=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build)
      build=1
      shift
      ;;
    --image)
      image="${2:?--image requires a value}"
      image_set=1
      shift 2
      ;;
    --out-dir)
      out_dir="${2:?--out-dir requires a value}"
      shift 2
      ;;
    --version)
      version="${2:?--version requires a value}"
      if [[ "${GLITCH_DEPLOY_ZIP_NAME:-}" == "" && "$zip_name_set" == "0" ]]; then
        zip_name="veloren-glitch-streamed-native-${version}.zip"
      fi
      if [[ "${GLITCH_DEPLOY_IMAGE:-}" == "" && "$image_set" == "0" ]]; then
        image="veloren-glitch-web:${version}"
      fi
      shift 2
      ;;
    --zip-name)
      zip_name="${2:?--zip-name requires a value}"
      zip_name_set=1
      shift 2
      ;;
    --test-install-id)
      test_install_id="${2:?--test-install-id requires a value}"
      shift 2
      ;;
    --test-title-token)
      test_title_token="${2:?--test-title-token requires a value}"
      shift 2
      ;;
    --skip-secret-scan)
      skip_secret_scan=1
      shift
      ;;
    --upload-command)
      upload_command="${2:?--upload-command requires a value}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "Unknown option: $1"
      ;;
  esac
done

zip_path="${out_dir%/}/${zip_name}"
manifest_path="${repo_root}/glitch-streamed-native.json"

[[ -f "$manifest_path" ]] || fail "Missing glitch-streamed-native.json"
[[ -f docker/Dockerfile.glitch-veloren-web ]] || fail "Missing docker/Dockerfile.glitch-veloren-web"
[[ -f docker/glitch-web-entrypoint.sh ]] || fail "Missing docker/glitch-web-entrypoint.sh"
[[ -f glitch/streamed-native/x11_mouse_bridge.py ]] || fail "Missing streamed-native X11 mouse bridge"
[[ -f glitch/streamed-native/inject_novnc_pointer_lock.py ]] || fail "Missing noVNC pointer-lock injector"
[[ -f glitch/streamed-native/novnc_pointer_lock_mouse.js ]] || fail "Missing noVNC pointer-lock browser script"

if [[ -n "$(git diff --name-only --diff-filter=U)" ]]; then
  fail "Unresolved merge conflicts are present. Resolve them before packaging."
fi

scan_for_secret() {
  local label="$1"
  local value="$2"

  [[ -n "$value" ]] || return 0

  log "Scanning packageable files for ${label}"
  if grep -R \
    --exclude-dir=.git \
    --exclude-dir=target \
    --exclude-dir=node_modules \
    --exclude='*.bak*' \
    --exclude='*.zip' \
    --exclude='.env' \
    --exclude='.env.*' \
    -nF "$value" .; then
    fail "${label} was found in packageable files. Remove it before deploying."
  fi
}

if [[ "$skip_secret_scan" != "1" ]]; then
  scan_for_secret TEST_INSTALL_ID "$test_install_id"
  scan_for_secret TEST_TITLE_TOKEN "$test_title_token"

  if [[ -z "$test_install_id" && -z "$test_title_token" ]]; then
    log "No TEST_INSTALL_ID or TEST_TITLE_TOKEN supplied; only structural ZIP checks will run."
  fi
else
  log "Secret scan skipped by request."
fi

if [[ "$build" == "1" ]]; then
  require_cmd docker
  log "Building streamed-native image: ${image}"
  docker buildx build \
    --platform linux/amd64 \
    --progress=plain \
    -f docker/Dockerfile.glitch-veloren-web \
    -t "$image" \
    --load \
    .
fi

require_cmd zip
require_cmd unzip
mkdir -p "$out_dir"
rm -f "$zip_path"

log "Writing upload ZIP: ${zip_path}"
zip -rq "$zip_path" . \
  -x ".git/*" \
  -x "target/*" \
  -x "**/target/*" \
  -x "node_modules/*" \
  -x "**/node_modules/*" \
  -x ".DS_Store" \
  -x "**/.DS_Store" \
  -x "*.bak*" \
  -x "**/*.bak*" \
  -x "*.zip" \
  -x ".env" \
  -x ".env.*" \
  -x "**/.env" \
  -x "**/.env.*" \
  -x "native-container.env" \
  -x "**/native-container.env" \
  -x "userdata/*" \
  -x "docker/userdata/*"

log "Verifying ZIP contents"
unzip -tq "$zip_path" >/dev/null
zip_listing="$(unzip -Z1 "$zip_path")"
zip_contains() {
  case $'\n'"$zip_listing"$'\n' in
    *$'\n'"$1"$'\n'*) return 0 ;;
    *) return 1 ;;
  esac
}

zip_contains "glitch-streamed-native.json" || fail "ZIP missing glitch-streamed-native.json"
zip_contains "docker/Dockerfile.glitch-veloren-web" || fail "ZIP missing streamed-native Dockerfile"
zip_contains "docker/glitch-web-entrypoint.sh" || fail "ZIP missing streamed-native entrypoint"
zip_contains "glitch/streamed-native/x11_mouse_bridge.py" || fail "ZIP missing X11 mouse bridge"
zip_contains "glitch/streamed-native/inject_novnc_pointer_lock.py" || fail "ZIP missing noVNC pointer-lock injector"
zip_contains "glitch/streamed-native/novnc_pointer_lock_mouse.js" || fail "ZIP missing noVNC pointer-lock browser script"

if [[ "$skip_secret_scan" != "1" ]]; then
  for secret in "$test_install_id" "$test_title_token"; do
    [[ -n "$secret" ]] || continue
    if grep -aF "$secret" < <(unzip -p "$zip_path" 2>/dev/null) >/dev/null; then
      fail "Verified ZIP still contains a supplied test secret."
    fi
  done
fi

log "ZIP ready: ${zip_path}"
ls -lh "$zip_path"

if [[ -n "$upload_command" ]]; then
  log "Running upload command"
  GLITCH_ZIP="$zip_path" \
    GLITCH_MANIFEST="$manifest_path" \
    GLITCH_DEPLOY_VERSION="$version" \
    bash -lc "$upload_command"
else
  log "No upload command configured. Upload this ZIP through the Glitch title build flow."
fi
