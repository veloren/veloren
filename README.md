# Nova-Forge

**Nova-Forge** is a standalone, LAN-first action-adventure RPG built on the open-source [Veloren](https://veloren.net) engine.
Play solo, host a game for friends on your local network, or run your own dedicated server — all **without an internet account or central authentication**.

> Built for people who want to just launch the game and play.

---

## What makes Nova-Forge different?

| Feature | Nova-Forge | Upstream Veloren |
|---|---|---|
| No account required | ✅ | ❌ (online auth) |
| Singleplayer | ✅ | ✅ |
| LAN co-op hosting | ✅ (built-in) | manual setup |
| Dedicated server | ✅ (auth-free) | requires auth |
| Public server browser | optional | ✅ |

Nova-Forge stubs out the Veloren authentication layer so that every play mode — singleplayer, LAN game, dedicated server — works out of the box with no external dependencies.

---

## Getting started

### Quick build & run

```bash
# Clone the repo
git clone https://github.com/shifty81/Nova-Forge.git
cd Nova-Forge

# Build and launch the game client
./nova-forge.sh run

# Or build & launch a dedicated server (LAN / local)
./nova-forge.sh server
```

See `./nova-forge.sh help` for all options.

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (version pinned in `rust-toolchain`)
- A GPU with Vulkan, Metal, or DX12 support
- Linux, macOS, or Windows

---

## Play modes

### Singleplayer
Launch the client, click **Singleplayer**, and you are in. No account, no server, no waiting.

### LAN Co-op
Click **Host LAN Game** to spin up a local server that friends on the same network can join immediately. No port-forwarding required for typical home networks.

### Dedicated server
```bash
./nova-forge.sh server
```
Runs a fully self-contained server. Players connect with any username — no external authentication.

---

## Building from source

```bash
# Dev build (fast iteration)
./nova-forge.sh build

# Optimised release build
./nova-forge.sh release

# Run tests
./nova-forge.sh test
```

---

## FAQ

### **Q:** Do I need an account?

**A:** No. Nova-Forge removes the mandatory authentication requirement. Any username works.

### **Q:** Is this compatible with official Veloren servers?

**A:** No. Nova-Forge modifies the auth protocol and is not intended for use with veloren.net servers.

### **Q:** How is this licensed?

**A:** Nova-Forge inherits the **[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0-standalone.html)** from Veloren. It is free to play, modify, and distribute.

### **Q:** What platforms are supported?

**A:** Linux (x86_64, ARM64), macOS, and Windows. x86_64 is the primary development target.

---

## Contributing

Issues and pull requests are welcome. Focus areas for Nova-Forge:

- LAN discovery and connection UX
- Auth-free server stability and compatibility
- Singleplayer world management
- Performance on lower-end hardware

---

## Credits

Nova-Forge is built on the shoulders of the [Veloren project](https://veloren.net) and its contributors:

- Software developers, artists, composers, and translators who built Veloren.
- The Veloren community for creating a rich open-source game engine and world.
