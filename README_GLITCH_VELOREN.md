# Veloren ⇄ Glitch Retrofit

This document records the work done to make **Veloren run inside Glitch as a browser-playable, Glitch-authenticated game**, without requiring players to manually create a second account or install a separate client.

The current implementation is intentionally **not forked upstream into Veloren yet**. It is a repo-local retrofit that lets us validate the product and deployment model first.

---

## 1. Goal

We needed Veloren to work with Glitch in a way that satisfies the product rules we use for every game on the platform:

1. The game should launch from Glitch.
2. Players should not manually register on another website.
3. Glitch should own the player identity handoff.
4. Glitch should track the install/session.
5. Veloren should use Glitch identity data where possible.
6. Browser play is required.
7. The solution should be generic enough to improve Glitch for future native/server games, not only Veloren.
8. We should not manually push the game image to a shared infrastructure image path as if Veloren were platform infrastructure.

The final direction is:

```text
Glitch Play Page
  -> starts/assigns a streamed native session
  -> auto-provisions Veloren auth credentials
  -> starts a native Veloren client inside Docker
  -> streams the game window to the browser through noVNC
  -> the native client connects to a Veloren dedicated server
```

For local testing, the server and streamed client currently run in one container using `VELOREN_WEB_MODE=all_in_one`.

For production, Glitch should split this into two roles:

```text
Shared dedicated Veloren server
  - One server can host multiple users
  - CPU-only is acceptable for the dedicated server

Per-user streamed native client
  - One streamed Voxygen client per browser session
  - Uses noVNC today
  - Should eventually move to WebRTC/GPU for production quality
```

---

## 2. Important architecture decision

Veloren is not a browser game by default.

The Veloren dedicated server exposes a native multiplayer protocol on port `14004`. That protocol cannot be played directly in a browser by visiting `http://server:14004` or `http://server:14005`.

The browser-play version needs a native Veloren client running somewhere. We chose:

```text
Veloren Voxygen native client
  -> Xvfb virtual display
  -> x11vnc
  -> websockify/noVNC
  -> browser URL
```

This is a proof-of-concept streaming stack. It works, but it is not a low-latency game-streaming codec.

Long-term production should use:

```text
Veloren Voxygen native client
  -> GPU rendering
  -> WebRTC streaming
  -> browser
```

The dedicated server can stay CPU-only and shared across users. The streamed native client is the expensive part.

---

## 3. What we built

The final repo state includes these important additions/changes:

```text
docker/Dockerfile.glitch-veloren
docker/Dockerfile.glitch-veloren-web
docker/glitch-entrypoint.sh
docker/glitch-web-entrypoint.sh
docker/docker-compose.glitch.yml
docker/docker-compose.web.yml
glitch/veloren-auth-provisioner/
glitch-streamed-native.json
patches/0001-veloren-glitch-auth-heartbeat.patch
patches/0002-veloren-voxygen-glitch-web-autoconnect.patch
```

The exact file set may differ depending on which retrofit package was applied, but those are the important concepts.

---

## 4. Dedicated server image

### Purpose

`docker/Dockerfile.glitch-veloren` builds the headless Veloren server only.

This is useful for:

- Dedicated server testing
- Future Glitch shared server capacity
- Separating server and streamed client roles in production

### Build

From the Veloren repo root:

```bash
docker buildx build \
  --platform linux/amd64 \
  --progress=plain \
  -f docker/Dockerfile.glitch-veloren \
  -t veloren-glitch-server:latest \
  --load .
```

### Run

```bash
docker run --rm -it \
  --platform linux/amd64 \
  -p 14004:14004/tcp \
  -p 14004:14004/udp \
  -p 14005:14005/tcp \
  -e GLITCH_API_BASE_URL="https://api.glitch.fun/api" \
  -e GLITCH_TITLE_ID="355282b1-f6a8-4183-9105-044993ba6066" \
  -e GLITCH_TITLE_TOKEN="7f478ff5-4871-4a2f-8944-1b5f1b201bd0.4wk3eFa6J0dmFKj3iCAfWsdctM4a3tma" \
  -e GLITCH_SHARED_PASSWORD="CHANGE_ME_SHARED_PASSWORD" \
  -e VELOREN_MAX_PLAYERS="32" \
  -v veloren-glitch-data:/opt/userdata \
  veloren-glitch-server:latest
```

### Ports

```text
14004/tcp - Veloren game server
14004/udp - Veloren game server UDP path if needed
14005/tcp - Veloren query/web service
```

Port `14005` is not the browser-playable game. It is only a small server/query endpoint.

---

## 5. Browser-streamed web image

### Purpose

`docker/Dockerfile.glitch-veloren-web` builds:

1. `veloren-server-cli`
2. `veloren-voxygen`
3. `veloren-auth-provisioner`
4. Runtime dependencies for Xvfb, Fluxbox, x11vnc, noVNC, websockify, Mesa/Vulkan, and audio libraries

This image can run:

```text
server_only
client_only
all_in_one
```

For local testing we use:

```text
VELOREN_WEB_MODE=all_in_one
```

For production, Glitch should run:

```text
server_container role: server_only
stream_client role: client_only
```

---

## 6. Build streamed web image

From the Veloren repo root:

```bash
docker buildx build \
  --platform linux/amd64 \
  --progress=plain \
  -f docker/Dockerfile.glitch-veloren-web \
  -t veloren-glitch-web:latest \
  --load .
```

The first full build can take a long time because it compiles Veloren. We added BuildKit cache mounts so later builds are much faster.

Expected successful image result:

```text
naming to docker.io/library/veloren-glitch-web:latest done
```

---

## 7. Run streamed browser version locally

```bash
docker run --rm -it \
  --platform linux/amd64 \
  -p 6080:6080/tcp \
  -p 14004:14004/tcp \
  -p 14004:14004/udp \
  -p 14005:14005/tcp \
  -e GLITCH_API_BASE_URL="https://api.glitch.fun/api" \
  -e GLITCH_TITLE_ID="355282b1-f6a8-4183-9105-044993ba6066" \
  -e GLITCH_TITLE_TOKEN="7f478ff5-4871-4a2f-8944-1b5f1b201bd0.4wk3eFa6J0dmFKj3iCAfWsdctM4a3tma" \
  -e GLITCH_INSTALL_ID="5663bdf5-c4e2-48a3-b06d-bc6a57befe63" \
  -e GLITCH_SHARED_PASSWORD="CHANGE_ME_SHARED_PASSWORD" \
  -e VELOREN_AUTH_MODE="official" \
  -e VELOREN_AUTH_AUTOREGISTER="1" \
  -e VELOREN_AUTH_SERVER_URL="https://auth.veloren.net" \
  -e VELOREN_AUTH_PASSWORD_SECRET="CHANGE_TO_STABLE_RANDOM_SECRET" \
  -e VELOREN_MAX_PLAYERS="32" \
  -e VELOREN_WEB_MODE="all_in_one" \
  -e VELOREN_SERVER_GRACE_SECONDS="0" \
  -e VELOREN_STREAM_PRESET="balanced" \
  -v veloren-glitch-web-data:/opt/userdata \
  veloren-glitch-web:latest
```

Open:

```text
http://localhost:6080/vnc.html?autoconnect=1&resize=scale&quality=8&compression=1&shared=1
```

---

## 8. Environment variables

### Required Glitch variables

```text
GLITCH_API_BASE_URL
GLITCH_TITLE_ID
GLITCH_TITLE_TOKEN
GLITCH_INSTALL_ID
GLITCH_SHARED_PASSWORD
```

### Important Veloren auth variables

```text
VELOREN_AUTH_MODE
VELOREN_AUTH_AUTOREGISTER
VELOREN_AUTH_SERVER_URL
VELOREN_AUTH_PASSWORD_SECRET
```

Supported auth modes:

```text
glitch
official
custom
none
```

For auto-registration against the Veloren auth service:

```bash
-e VELOREN_AUTH_MODE="official"
-e VELOREN_AUTH_AUTOREGISTER="1"
-e VELOREN_AUTH_SERVER_URL="https://auth.veloren.net"
-e VELOREN_AUTH_PASSWORD_SECRET="CHANGE_TO_STABLE_RANDOM_SECRET"
```

### Stream variables

```text
VELOREN_WEB_MODE=all_in_one|client_only|server_only
VELOREN_WEB_PORT=6080
VELOREN_VNC_PORT=5900
VELOREN_STREAM_PRESET=performance|balanced|quality|custom
VELOREN_WEB_WIDTH
VELOREN_WEB_HEIGHT
VELOREN_VNC_QUALITY
VELOREN_NOVNC_QUALITY
VELOREN_VNC_COMPRESS_LEVEL
VELOREN_NOVNC_COMPRESSION
VELOREN_VNC_WAIT_MS
VELOREN_VNC_DEFER_MS
VELOREN_VNC_NCACHE
VELOREN_BROWSER_RESIZE_MODE
VELOREN_SERVER_GRACE_SECONDS
```

---

## 9. Stream presets

### Performance

Lower resolution, smoother:

```bash
-e VELOREN_STREAM_PRESET="performance"
```

Defaults:

```text
960x540
VNC quality 6
compression 2
wait/defer 16ms
```

### Balanced

Default:

```bash
-e VELOREN_STREAM_PRESET="balanced"
```

Defaults:

```text
1280x720
VNC quality 8
compression 1
wait/defer 10ms
```

### Quality

Sharper but slower:

```bash
-e VELOREN_STREAM_PRESET="quality"
```

Defaults:

```text
1600x900
VNC quality 9
compression 1
wait/defer 8ms
```

### Custom

Example:

```bash
-e VELOREN_STREAM_PRESET="custom" \
-e VELOREN_WEB_WIDTH="1152" \
-e VELOREN_WEB_HEIGHT="648" \
-e VELOREN_VNC_QUALITY="8" \
-e VELOREN_VNC_COMPRESS_LEVEL="1" \
-e VELOREN_NOVNC_QUALITY="8" \
-e VELOREN_NOVNC_COMPRESSION="1"
```

---

## 10. Glitch validation and identity

Originally, the Veloren retrofit used the Glitch install ID directly as the Veloren username.

We later changed this because the Glitch validation response returns a display username. The desired behavior became:

```text
Glitch install_id = stable identity / login seed
Glitch validate response user_name = display name source
Veloren UUID = derived from install_id
```

This avoids breaking saves if the user display name changes.

The patch introduced a Glitch auth path in Veloren server login code. It validates the Glitch install ID against Glitch and uses the returned user name where appropriate.

---

## 11. Auto-registration

Auto-registration became a hard requirement.

The final behavior:

1. The user signs into Glitch only.
2. Glitch provides an install ID.
3. The streamed web container runs `veloren-auth-provisioner`.
4. The provisioner validates/uses Glitch identity.
5. It generates a deterministic Veloren username.
6. It generates a deterministic password using `VELOREN_AUTH_PASSWORD_SECRET`.
7. It registers the account on the configured Veloren auth server.
8. If the account already exists, it logs in using the deterministic password.
9. Voxygen launches using the provisioned Veloren username/password.

Example log from the working run:

```text
[glitch-web] Auto-provisioned Veloren username: blackmage-4d00a876b1 from Glitch install 5663bdf5-c4e2-48a3-b06d-bc6a57befe63
```

Important: keep `VELOREN_AUTH_PASSWORD_SECRET` stable forever. If it changes, Glitch will no longer be able to derive the same Veloren password for already-created accounts.

### Production warning

For production, prefer a Glitch-owned Veloren auth service:

```text
https://veloren-auth.glitch.fun
```

Using the public Veloren auth service may work technically, but it introduces product, policy, rate-limit, and dependency risks.

---

## 12. Voxygen autoconnect patch

Veloren’s normal client expects interactive login. For browser play, we needed non-interactive login.

The autoconnect patch lets the container pass login details through environment variables:

```text
VELOREN_GLITCH_AUTOCONNECT=1
VELOREN_SERVER_ADDRESS=127.0.0.1:14004
GLITCH_INSTALL_ID=<username or install id>
GLITCH_SHARED_PASSWORD=<password>
```

When auto-registration is enabled, the entrypoint intentionally rewrites:

```text
GLITCH_INSTALL_ID -> provisioned Veloren username
GLITCH_SHARED_PASSWORD -> provisioned Veloren password
```

This lets the same autoconnect path work for both Glitch-auth mode and official/custom Veloren-auth mode.

---

## 13. noVNC browser URL

The browser stream URL is:

```text
http://localhost:6080/vnc.html?autoconnect=1&resize=scale&quality=8&compression=1&shared=1
```

`6080` is the web/noVNC port.

`5900` is the internal VNC port.

`14004` is the Veloren game server port.

`14005` is Veloren's server query/web service.

---

## 14. The wallpaper popup issue

A popup appeared inside the streamed desktop:

```text
fbsetbg: I can't find an app to set the wallpaper with...
```

This was caused by Fluxbox trying to restore or set a wallpaper through `fbsetbg`.

We fixed this by:

1. Creating a private Fluxbox config.
2. Setting a plain black root window.
3. Replacing `/usr/bin/fbsetbg` with a no-op script inside the container.
4. Providing a harmless `xmessage` fallback if needed.

The key runtime block:

```bash
disable_fluxbox_wallpaper_popup() {
  if [[ -x /usr/bin/fbsetbg && ! -f /usr/bin/fbsetbg.real ]]; then
    mv /usr/bin/fbsetbg /usr/bin/fbsetbg.real || true
    cat > /usr/bin/fbsetbg <<'SH'
#!/bin/sh
exit 0
SH
    chmod +x /usr/bin/fbsetbg || true
  fi

  if ! command -v xmessage >/dev/null 2>&1; then
    cat > /usr/local/bin/xmessage <<'SH'
#!/bin/sh
exit 0
SH
    chmod +x /usr/local/bin/xmessage || true
    export PATH="/usr/local/bin:${PATH}"
  fi
}
```

This function must run before Fluxbox starts.

---

## 15. x11vnc startup problem and final fix

When we tried to tune `x11vnc` directly with quality/compression flags, it failed to open port `5900`:

```text
Timed out waiting for TCP 127.0.0.1:5900
```

The final entrypoint uses a robust VNC startup:

1. Try tuned x11vnc first.
2. Wait up to 12 seconds.
3. If it fails, kill it.
4. Print the failed log.
5. Start minimal known-working x11vnc.
6. Continue.

This preserves the browser-play flow even when a specific x11vnc build rejects a tuning flag.

Important functions:

```bash
start_x11vnc_tuned()
start_x11vnc_minimal()
start_x11vnc_robust()
```

---

## 16. Current black screen behavior

Setting:

```bash
-e VELOREN_SERVER_GRACE_SECONDS="0"
```

removes the explicit post-server-ready wait.

However, there may still be a long black screen.

That black screen is likely not the grace delay anymore. It is probably one or more of:

1. noVNC page opening before Voxygen has drawn its first real frame.
2. Voxygen compiling shaders on startup.
3. CPU rendering through Mesa llvmpipe.
4. Veloren loading assets/world/client state.
5. Browser stream connected to an empty X display before the game window is ready.

The logs show Voxygen uses CPU rendering:

```text
llvmpipe (LLVM 15.0.6, 128 bits)
device_type=Cpu
```

That is the main performance limitation.

### Future black-screen fixes

Potential improvements:

1. Start Voxygen before exposing the final play URL.
2. Have Glitch show a loading page until Voxygen logs that rendering has started.
3. Add a lightweight readiness detector that waits for the Voxygen window to exist.
4. Use `xdotool` or `wmctrl` to wait for/map/focus the Voxygen window.
5. Use a simple splash screen or loading image in the X display before Voxygen starts.
6. Move from noVNC to WebRTC streaming.
7. Use GPU-backed rendering for the streamed client.

---

## 17. Performance limitations

Current browser play works, but quality is limited.

The current stack is:

```text
Voxygen -> llvmpipe CPU Vulkan -> Xvfb -> x11vnc -> noVNC -> browser
```

Known performance constraints:

1. CPU rendering is slow for 3D games.
2. VNC is not optimized for fast 3D game streaming.
3. noVNC image quality depends on compression and browser scaling.
4. High resolution increases CPU load.
5. Quality mode can look sharper but may lag more.

For production, the better stack is:

```text
Voxygen -> GPU -> WebRTC -> browser
```

---

## 18. Why not manually push to glitchgames.azurecr.io?

We explicitly decided not to manually push:

```text
glitchgames.azurecr.io/veloren-glitch-server:latest
```

That registry is infrastructure. Veloren should be handled as a game build artifact uploaded through Glitch, not as a manually managed shared platform image.

Correct model:

```text
Developer uploads game ZIP/build to Glitch
  -> Glitch builds/imports the Docker image internally
  -> Glitch tags it title/build specifically
  -> Glitch deploys server/client roles
```

Example internal tag shape:

```text
glitchgames.azurecr.io/titles/{title_id}/builds/{build_id}:{version}
```

Developers should not manually push these images.

---

## 19. Generic Glitch platform improvement

Veloren exposed a missing platform capability: Glitch needs first-class support for native streamed games and server-container games.

Recommended generic deployment types:

```text
server_container
streamed_native
```

### server_container

For games that expose a multiplayer server:

```json
{
  "deployment_type": "server_container",
  "runtime": "docker",
  "ports": [
    { "port": 14004, "protocol": "tcp", "public": true },
    { "port": 14004, "protocol": "udp", "public": true }
  ],
  "capacity": {
    "max_players": 32,
    "allocation_mode": "shared_server"
  }
}
```

### streamed_native

For games that need a native client streamed into a browser:

```json
{
  "deployment_type": "streamed_native",
  "server": {
    "type": "server_container",
    "allocation_mode": "shared_server"
  },
  "client": {
    "type": "stream_container",
    "allocation_mode": "one_per_user",
    "stream_protocol": "novnc"
  }
}
```

The play response should look like:

```json
{
  "type": "streamed_native",
  "url": "https://play.glitch.fun/streams/<session>/vnc.html?autoconnect=1&resize=scale",
  "install_id": "<glitch_install_id>",
  "stream_protocol": "novnc"
}
```

---

## 20. Matchmaker notes

We did not update the existing Glitch matchmaker yet.

The current matchmaker is designed around exclusive session assignment, which is correct for Pixel Streaming but wrong for shared multiplayer servers.

For Veloren:

```text
One dedicated server can support many players.
One streamed client container is one player/session.
```

Therefore:

```text
Pixel Streaming games -> existing matchmaker
Server-container multiplayer games -> capacity-aware server allocator
Streamed native games -> one stream client per session + shared server allocation
```

Later, Glitch can add a matchmaker v2 that supports capacity-based allocation.

---

## 21. Save/progression notes

The safest current model:

```text
Veloren live saves stay server-side in persistent volume
Glitch install/session tracks identity
Glitch Cloud Save can be used later for snapshots/backups
```

Do not try to replace Veloren’s live SQLite persistence with Glitch Cloud Save without a deeper import/export design.

---

## 22. Leaderboard notes

We added helper logic earlier for Glitch leaderboard submission, but we have not finalized which Veloren stats should map to Glitch leaderboards.

Possible leaderboard stats:

```text
monster kills
PvP kills
wealth
distance traveled
play minutes
highest level
boss kills
crafted items
gathered resources
```

This should be a product decision before wiring final stat hooks.

---

## 23. Current working run command

```bash
docker run --rm -it \
  --platform linux/amd64 \
  -p 6080:6080/tcp \
  -p 14004:14004/tcp \
  -p 14004:14004/udp \
  -p 14005:14005/tcp \
  -e GLITCH_API_BASE_URL="https://api.glitch.fun/api" \
  -e GLITCH_TITLE_ID="355282b1-f6a8-4183-9105-044993ba6066" \
  -e GLITCH_TITLE_TOKEN="7f478ff5-4871-4a2f-8944-1b5f1b201bd0.4wk3eFa6J0dmFKj3iCAfWsdctM4a3tma" \
  -e GLITCH_INSTALL_ID="5663bdf5-c4e2-48a3-b06d-bc6a57befe63" \
  -e GLITCH_SHARED_PASSWORD="CHANGE_ME_SHARED_PASSWORD" \
  -e VELOREN_AUTH_MODE="official" \
  -e VELOREN_AUTH_AUTOREGISTER="1" \
  -e VELOREN_AUTH_SERVER_URL="https://auth.veloren.net" \
  -e VELOREN_AUTH_PASSWORD_SECRET="CHANGE_TO_STABLE_RANDOM_SECRET" \
  -e VELOREN_MAX_PLAYERS="32" \
  -e VELOREN_WEB_MODE="all_in_one" \
  -e VELOREN_SERVER_GRACE_SECONDS="0" \
  -e VELOREN_STREAM_PRESET="balanced" \
  -v veloren-glitch-web-data:/opt/userdata \
  veloren-glitch-web:latest
```

Open:

```text
http://localhost:6080/vnc.html?autoconnect=1&resize=scale&quality=8&compression=1&shared=1
```

---

## 24. Known warnings that are acceptable for now

These appeared in logs and are not currently blocking:

```text
XRandR reported that the display's 0mm in size
Unable to find extension: VK_EXT_physical_device_drm
Unable to find extension: VK_EXT_swapchain_colorspace
VK_EXT_memory_budget is not available
Missing downlevel flags
slow system execution millis=...
```

The important thing is whether:

1. noVNC starts.
2. Voxygen starts.
3. The server reaches ready.
4. The server accepts TCP.
5. The auth account is provisioned.
6. The player logs in.
7. The browser receives the stream.

---

## 25. Troubleshooting

### Docker build is very slow

Expected on first build.

Use BuildKit:

```bash
docker buildx build --platform linux/amd64 --progress=plain ...
```

Clean Docker cache only if Docker becomes corrupted or out of disk:

```bash
docker system df
docker buildx du
docker buildx prune -af
docker builder prune -af
```

If Docker hangs on prune, restart Docker Desktop.

### Docker build fails with `Cargo.lock needs to be updated but --locked was passed`

Remove `--locked` from the Dockerfile build step or regenerate `Cargo.lock`.

### Docker build fails with `aarch64-linux-gnu-gcc` / `cannot find ld`

Build explicitly for linux/amd64:

```bash
docker buildx build --platform linux/amd64 ...
```

Also ensure the Dockerfile has linker tooling:

```text
build-essential
binutils
mold
lld
gcc-aarch64-linux-gnu
binutils-aarch64-linux-gnu
```

### Docker build fails with `Read-only file system` or Docker metadata DB I/O error

Docker Desktop storage is likely wedged or out of space.

Fix:

```bash
docker buildx prune -af
docker builder prune -af
docker system prune -af --volumes
```

Then restart Docker Desktop and increase Docker disk image size.

### Runtime fails with `libxkbcommon-x11.so could not be loaded`

The runtime image is missing X11 keyboard libraries.

Required package:

```text
libxkbcommon-x11-0
```

The runtime Dockerfile should also include:

```text
xkb-data
libxkbcommon0
libX11
libxcb
libvulkan
libGL
```

### Runtime fails waiting for `127.0.0.1:5900`

x11vnc did not start.

Use the robust `start_x11vnc_robust` entrypoint that falls back to the minimal x11vnc command.

### Popup says it cannot set wallpaper

Fluxbox is calling `fbsetbg`.

Run `disable_fluxbox_wallpaper_popup` before starting Fluxbox.

### Game connects but looks slow/pixelated

Expected with CPU rendering and noVNC.

Try:

```bash
-e VELOREN_STREAM_PRESET="performance"
```

or:

```bash
-e VELOREN_STREAM_PRESET="quality"
```

For production, move to GPU/WebRTC.

### Server panics with `Arc<World>`

This happened when the server was built with `--no-default-features`.

Fix:

```bash
cargo build --release -p veloren-server-cli
```

Do not build the server with `--no-default-features`.

The client can still be built with:

```bash
cargo build --release -p veloren-voxygen --no-default-features
```

---

## 26. Files to commit

Before pushing, check:

```bash
git status --short
```

Likely files to include:

```text
docker/Dockerfile.glitch-veloren
docker/Dockerfile.glitch-veloren-web
docker/glitch-entrypoint.sh
docker/glitch-web-entrypoint.sh
docker/docker-compose.glitch.yml
docker/docker-compose.web.yml
glitch/veloren-auth-provisioner/Cargo.toml
glitch/veloren-auth-provisioner/src/main.rs
glitch-streamed-native.json
README_GLITCH.md
```

Likely generated files to avoid:

```text
target/
Docker build cache
runtime userdata
Veloren server database files
logs
```

Add to `.gitignore` if needed:

```gitignore
target/
veloren-glitch-data/
veloren-glitch-web-data/
*.log
```

Docker named volumes are not in the repo, but avoid copying their contents into the repo.

---

## 27. Recommended commit message

```text
feat: add Glitch streamed web support for Veloren
```

Optional longer message:

```text
Adds Glitch-authenticated Veloren server and streamed web client support.

- Adds Docker builds for dedicated server and browser-streamed native client
- Adds noVNC/Xvfb/x11vnc streaming entrypoint
- Adds Glitch install validation and autoconnect flow
- Adds Veloren auth auto-provisioning helper
- Adds Fluxbox/noVNC runtime fixes
- Keeps Veloren server build default features enabled
```

---

## 28. Current status

Working:

```text
Docker build succeeds
Dedicated server starts
Browser stream opens
Voxygen starts in Xvfb
Auto-registration provisions Veloren username
Voxygen connects to local dedicated server
Server accepts client connection
```

Still needs product/platform work:

```text
Reduce black screen before first visible frame
Improve performance and picture quality
Move from noVNC to WebRTC/GPU for production
Add first-class Glitch deployment_type=streamed_native
Add shared server allocator for server_container games
Decide final leaderboard/stat mapping
Decide final save snapshot strategy
```

---

## 29. Summary

We turned Veloren from a native-only multiplayer game into a Glitch-runnable streamed browser experience.

The current implementation proves:

1. Veloren can run in Docker.
2. The dedicated server can run CPU-only.
3. A native Voxygen client can run headlessly in Docker.
4. The native client can be streamed to the browser through noVNC.
5. Glitch identity can drive the login flow.
6. Veloren auth accounts can be auto-provisioned.
7. The model can become a generic Glitch feature for streamed native games.

This should be pushed to the repo as an integration branch, not forked upstream into Veloren yet.
