# Glitch Streamed-Native Integration

This repository contains the Veloren streamed-native integration for Glitch. It is one game-specific adapter on top of Glitch's game-agnostic deployment flow.

Glitch is not just for this MMO. Glitch is designed for any uploaded game that needs browser play, identity, access control, analytics, matchmaking, cloud deployment, and save persistence. This repo is the Veloren/native-game reference implementation of that pattern.

## What this implementation does

This build lets Veloren run as a native Linux game inside a Docker container, stream to the browser through noVNC, and authenticate the player through Glitch runtime identity.

```text
Glitch play page
→ /api/titles/{title_id}/play
→ matchmaker wakes or allocates a streamed-native VM
→ VM Caddy/proxy points to the container public port
→ container nginx serves noVNC, websockify, and audio on one public origin
→ Veloren server/client run inside the container
→ Glitch install/session IDs identify the player
→ optional Glitch save APIs archive/restore game userdata
```

## Production safety rule

The matchmaker and VM proxy are production infrastructure. Avoid changing them for game-specific features unless the change is truly platform-level.

The current safer design keeps audio inside the Veloren container:

```text
VM Caddy / ps-* proxy
→ container public :6080

Inside the container:
  nginx :6080
    /vnc.html and /websockify → noVNC/websockify :6082
    /glitch-audio*           → audio server :6081

Veloren audio:
  Voxygen → per-user PulseAudio null sink
  ffmpeg  → captures glitch_stream_sink.monitor
  browser → /glitch-audio.webm
```

This keeps PS1/PS2 Pixel Streaming unaffected.

## Key files

```text
docker/Dockerfile.glitch-veloren-web
  Builds the streamed-native image.
  Installs runtime dependencies for Xvfb, x11vnc, noVNC, websockify, nginx,
  PulseAudio, ffmpeg, and the non-root audio runtime user.
  Enables the optional Rust features needed for Glitch: server-cli
  `glitch-auth` and voxygen `glitch-web`.

docker/glitch-web-entrypoint.sh
  Main streamed-native boot script.
  Starts the virtual display, noVNC, nginx proxy, PulseAudio user daemon,
  local Veloren server, Voxygen client, cloud-save helpers, and audio stream.

glitch/streamed-native
  Browser-streamed helper assets copied only into the Glitch web image,
  including the noVNC pointer-lock injector and X11 mouse bridge.

voxygen/src/window.rs
  Contains the noVNC mouse compatibility patch behind the optional
  `glitch-web` feature. Normal Voxygen builds do not compile this path.

glitch/veloren-auth-provisioner
  Provisions or maps a Veloren login identity from Glitch runtime identity.
```

## Optional build gates

The Veloren code changes are opt-in:

```text
server-cli feature: glitch-auth
  Enables glitch:// server auth and the optional reqwest dependency.

voxygen feature: glitch-web
  Enables Glitch browser-stream autoconnect and noVNC absolute mouse handling.
  This also enables the client-side glitch-auth login adapter.
```

The Glitch Docker image enables both features. Regular Veloren builds keep the
upstream defaults.

## Runtime environment variables

Provide these through Glitch deployment/runtime configuration. Do not commit real tokens or install IDs.

```text
GLITCH_API_BASE_URL=https://api.glitch.fun/api
GLITCH_TITLE_ID=<title uuid>
GLITCH_TITLE_TOKEN=<title token>
GLITCH_SHARED_PASSWORD=<shared runtime password>

VELOREN_WEB_MODE=all_in_one
VELOREN_AUTH_MODE=official
VELOREN_AUTH_SERVER_URL=https://auth.veloren.net
VELOREN_AUTH_AUTOREGISTER=1
VELOREN_SERVER_GRACE_SECONDS=0
VELOREN_STREAM_PRESET=balanced
```

Player/session values:

```text
GLITCH_INSTALL_ID=<player install id>
GLITCH_USER_INSTALL_ID=<player install id>
GLITCH_SESSION_ID=<play session id>
```

Important: `GLITCH_INSTALL_ID` may be missing when the VM/container first boots. The container can start in idle mode and wait for the browser/noVNC URL to provide install/session identity.

Stream/audio values:

```text
GLITCH_PUBLIC_PORT=6080
GLITCH_NOVNC_INTERNAL_PORT=6082
GLITCH_AUDIO_ENABLED=1
GLITCH_AUDIO_PORT=6081
GLITCH_AUDIO_SINK=glitch_stream_sink
GLITCH_AUDIO_BITRATE=96000
GLITCH_AUDIO_SAMPLE_RATE=48000
GLITCH_AUDIO_CHANNELS=2
```

Local/software rendering values:

```text
VELOREN_ENABLE_GPU=0
LIBGL_ALWAYS_SOFTWARE=1
NVIDIA_VISIBLE_DEVICES=none
NVIDIA_DRIVER_CAPABILITIES=compute,utility
```

## Identity and account behavior

Glitch identity is based on install/session data, not a random local disk user.

The integration maps a stable Glitch install ID to a deterministic Veloren username. The same Glitch player should return to the same Veloren identity when the same install/user ID and save/server state are restored.

Do not hardcode test install IDs. Pass them only through local env files or runtime variables.

## Saves

The streamed-native container has cloud-save helper logic in `docker/glitch-web-entrypoint.sh`.

Intent:

```text
Before game launch:
  check Glitch save slot
  restore archived userdata if available

During/after play:
  periodically archive and upload userdata
  upload again on normal shutdown
```

Local container disk is temporary. Glitch save APIs are the intended persistent storage path for browser streamed-native sessions.

## Audio implementation

noVNC does not carry game audio by itself. This integration adds a separate browser audio stream on the same origin as noVNC.

Current implementation:

```text
PulseAudio:
  runs as non-root user: glitch
  uses normal per-user daemon mode
  creates null sink: glitch_stream_sink

Veloren Voxygen:
  runs as glitch
  connects through PULSE_SERVER=unix:/tmp/glitch-pulse/native

ffmpeg audio streamer:
  runs as glitch
  captures glitch_stream_sink.monitor
  serves /glitch-audio.webm and /glitch-audio-status
```

The browser audio player is injected into noVNC. If autoplay is blocked, the page shows an **Enable game audio** button.

## Mouse/camera implementation

noVNC/VNC is not a true native relative mouse transport. Third-person games often expect relative pointer movement and pointer lock, while VNC normally sends absolute pointer positions.

Best-known current state:

```text
GLITCH_NOVNC_CAMERA_MOUSE_FIX_V1 restored in voxygen/src/window.rs
GLITCH_VNC_ABSOLUTE_MOUSE defaults to 1 in docker/glitch-web-entrypoint.sh
GLITCH_NOVNC_POINTER_LOCK defaults to 1 and injects a noVNC browser patch
GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE / Y_SCALE tune streamed camera sensitivity
Default streamed mouse scale is currently X=0.015, Y=0.006
Pointer-locked noVNC sends center-relative motion into Veloren. If pointer lock
is not available, the fallback derives motion from consecutive real absolute
events and ignores cursor recenter/focus warps.
The noVNC bridge catches async pointer-lock rejections and waits briefly after
lock exit before asking Chrome to capture the mouse again.
The stream also starts a same-origin X11 mouse WebSocket bridge as a fallback:
pointer-locked noVNC movement is mirrored into XTest relative mouse motion for
older Voxygen binaries that do not include the Rust-side noVNC cursor patch.
Later over-tuned V3 mouse mapping was reverted
```

When the stream is embedded, the parent iframe must allow pointer lock:

```html
<iframe
  src="https://.../vnc.html?autoconnect=1&resize=scale&quality=8&compression=1&shared=1"
  allow="pointer-lock; fullscreen"
></iframe>
```

Without `allow="pointer-lock"`, the browser may deny mouse capture. The noVNC
patch suppresses raw absolute mousemove events while unlocked so the camera does
not keep receiving iframe-edge coordinates.

The injected page script also exposes an integration point for wrapper/bridge
scripts such as Aegis:

```js
window.__glitchNoVNCPointerLockMouseV1.sendRelativeMouse(dx, dy, buttonMask);

window.dispatchEvent(new CustomEvent("aegis:mouse-delta", {
  detail: { dx, dy, buttonMask }
}));
```

Supported external delta event names are `aegis:mouse-delta`,
`aegis:pointer-delta`, `aegis-bridge:mouse-delta`,
`aegis-bridge:pointer-delta`, `glitch:mouse-delta`,
`glitch:pointer-delta`, and `glitch-novnc-pointer-delta`. The script also
dispatches `glitch:novnc-pointer-lock-loading`,
`glitch:novnc-pointer-lock-ready`, `glitch:novnc-pointer-lock-active`,
`glitch:novnc-pointer-lock-released`, and
`glitch:novnc-pointer-lock-error`.

Check mouse patch state:

```bash
grep -n "GLITCH_NOVNC_CAMERA_MOUSE_FIX\|GLITCH_NOVNC_POINTER_LOCK\|GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE" voxygen/src/window.rs || true

grep -n "GLITCH_NOVNC_POINTER_LOCK\|GLITCH_VNC_ABSOLUTE_MOUSE\|GLITCH_REVERT_NOVNC_MOUSE_OPTIMAL\|GLITCH_MOUSE_V1_LITE_RESTORE" docker/glitch-web-entrypoint.sh || true

grep -n "GLITCH_NOVNC_POINTER_LOCK_MOUSE_V1" glitch/streamed-native/novnc_pointer_lock_mouse.js || true
```

Preview the noVNC sensitivity math without building or launching the game:

```bash
scripts/glitch-mouse-sensitivity.sh

GLITCH_VNC_ABSOLUTE_MOUSE_X_SCALE=0.007 \
GLITCH_VNC_ABSOLUTE_MOUSE_Y_SCALE=0.003 \
scripts/glitch-mouse-sensitivity.sh
```

## Local Docker test

Create a temporary local env file outside the repo:

```bash
cat > /tmp/veloren-local-glitch.env <<'EOF'
GLITCH_API_BASE_URL=https://api.glitch.fun/api
GLITCH_TITLE_ID=<title uuid>
GLITCH_TITLE_TOKEN=<title token>

GLITCH_INSTALL_ID=<test install id>
GLITCH_USER_INSTALL_ID=<test install id>
GLITCH_SESSION_ID=<test session id>

GLITCH_SHARED_PASSWORD=<non-empty local value>

VELOREN_WEB_MODE=all_in_one
VELOREN_AUTH_MODE=official
VELOREN_AUTH_SERVER_URL=https://auth.veloren.net
VELOREN_AUTH_AUTOREGISTER=1
VELOREN_SERVER_GRACE_SECONDS=0
VELOREN_STREAM_PRESET=balanced

VELOREN_ENABLE_GPU=0
LIBGL_ALWAYS_SOFTWARE=1
NVIDIA_VISIBLE_DEVICES=none
NVIDIA_DRIVER_CAPABILITIES=compute,utility

GLITCH_PUBLIC_PORT=6080
GLITCH_NOVNC_INTERNAL_PORT=6082
GLITCH_AUDIO_ENABLED=1
GLITCH_AUDIO_PORT=6081
GLITCH_AUDIO_SINK=glitch_stream_sink
GLITCH_AUDIO_BITRATE=96000
GLITCH_AUDIO_SAMPLE_RATE=48000
GLITCH_AUDIO_CHANNELS=2
EOF

chmod 600 /tmp/veloren-local-glitch.env
```

Build locally:

```bash
docker buildx build \
  --platform linux/amd64 \
  -f docker/Dockerfile.glitch-veloren-web \
  -t veloren-glitch-container-audio-proxy-test:local \
  --load \
  .
```

Run locally:

```bash
docker rm -f veloren-local-glitch-test 2>/dev/null || true

docker run \
  --platform linux/amd64 \
  --name veloren-local-glitch-test \
  --env-file /tmp/veloren-local-glitch.env \
  -p 6080:6080 \
  -p 6081:6081 \
  -p 6082:6082 \
  -p 14004:14004 \
  veloren-glitch-container-audio-proxy-test:local
```

Open through the public container proxy, not the internal noVNC port:

```text
http://127.0.0.1:6080/vnc.html?autoconnect=1&resize=scale&quality=8&compression=1&shared=1&HoveringMouse=true&install_id=<test install id>&session_id=<test session id>&path=websockify%3Finstall_id%3D<test install id>%26user_install_id%3D<test install id>%26glitch_install_id%3D<test install id>%26session_id%3D<test session id>
```

## Local verification commands

```bash
curl -I --max-time 8 http://127.0.0.1:6080/vnc.html
curl -sS --max-time 8 http://127.0.0.1:6080/glitch-audio-status | python3 -m json.tool
curl -I --max-time 8 http://127.0.0.1:6082/vnc.html
curl -sS --max-time 8 http://127.0.0.1:6081/glitch-audio-status | python3 -m json.tool
```

Expected:

```text
6080/vnc.html works through nginx
6080/glitch-audio-status returns JSON through nginx
6082/vnc.html works directly to noVNC
6081/glitch-audio-status returns JSON directly from the audio server
```

Deep diagnostics:

```bash
docker exec -it veloren-local-glitch-test bash -lc '
echo "=== users/dirs ==="
id glitch || true
ls -ld /home/glitch /tmp/glitch-pulse /tmp/glitch-xdg-runtime /tmp/veloren-web || true

echo
echo "=== pulse ==="
gosu glitch env HOME=/home/glitch XDG_RUNTIME_DIR=/tmp/glitch-xdg-runtime PULSE_RUNTIME_PATH=/tmp/glitch-pulse PULSE_SERVER=unix:/tmp/glitch-pulse/native pactl info || true
gosu glitch env HOME=/home/glitch XDG_RUNTIME_DIR=/tmp/glitch-xdg-runtime PULSE_RUNTIME_PATH=/tmp/glitch-pulse PULSE_SERVER=unix:/tmp/glitch-pulse/native pactl list short sinks || true
gosu glitch env HOME=/home/glitch XDG_RUNTIME_DIR=/tmp/glitch-xdg-runtime PULSE_RUNTIME_PATH=/tmp/glitch-pulse PULSE_SERVER=unix:/tmp/glitch-pulse/native pactl list short sources || true

echo
echo "=== logs ==="
tail -n 120 /tmp/veloren-web/glitch-audio-server.log 2>/dev/null || true
tail -n 120 /tmp/veloren-web/glitch-nginx-error.log 2>/dev/null || true
tail -n 120 /tmp/veloren-web/glitch-nginx-access.log 2>/dev/null || true

echo
echo "=== ports ==="
ss -lntp | grep -E ":6080|:6081|:6082|:14004" || true
'
```

## Packaging for upload

Before zipping, scan for test IDs and tokens. Replace the values below with whatever was used locally.

```bash
cd /Users/devindixon/Development/Glitch-Games-Veloren

TEST_INSTALL_ID="<test install id>"
TEST_TITLE_TOKEN="<test title token>"

if grep -R \
  --exclude-dir=.git \
  --exclude-dir=target \
  --exclude-dir=node_modules \
  --exclude='*.bak*' \
  --exclude='*.zip' \
  -nE "$TEST_INSTALL_ID|$TEST_TITLE_TOKEN" .; then
  echo "FAIL: test install_id or title token found in repo. Do not package yet."
  exit 1
else
  echo "OK: no test install_id or test title token found in packageable repo files."
fi
```

Create the upload ZIP:

```bash
cd /Users/devindixon/Development/Glitch-Games-Veloren

OUT="/Users/devindixon/Downloads/veloren-glitch-streamed-native-user-pulse-audio-v1-$(date +%Y%m%d-%H%M%S).zip"

rm -f "$OUT"

zip -r "$OUT" . \
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
  -x "**/native-container.env"

echo "WROTE: $OUT"
ls -lh "$OUT"
```

Verify the ZIP:

```bash
ZIP="$(ls -t /Users/devindixon/Downloads/veloren-glitch-streamed-native-user-pulse-audio-v1-*.zip | head -1)"

if unzip -p "$ZIP" 2>/dev/null | grep -E "$TEST_INSTALL_ID|$TEST_TITLE_TOKEN"; then
  echo "FAIL: ZIP contains test install_id or token. Delete it and inspect."
  exit 1
else
  echo "OK: ZIP does not contain local test install_id or test title token."
fi

echo "READY TO UPLOAD:"
echo "$ZIP"
```

## Production deployment notes

Use Glitch title deployment variables for title IDs, tokens, shared passwords, and runtime settings.

Do not bake any of these into the ZIP:

```text
GLITCH_INSTALL_ID
GLITCH_USER_INSTALL_ID
GLITCH_SESSION_ID
GLITCH_TITLE_TOKEN
local test env files
native-container.env
```

The play URL should be generated by Glitch and the matchmaker. For streamed-native noVNC, it should include the install/session ID in the URL so the idle launcher can bind the browser session to the correct Glitch player.

## Known limitations

Audio now works through PulseAudio + ffmpeg + browser audio element, but browser autoplay may still require a click.

Mouse/camera is constrained by noVNC/VNC not being a true native relative mouse transport. The current V1 compatibility patch is the best-known version so far.

Local Docker Desktop on Apple Silicon runs this image through amd64 emulation. Performance may be choppy locally and is not representative of a proper GPU VM.
