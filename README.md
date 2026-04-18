# Nova-Forge

<!-- TODO: add project logo here -->

**Nova-Forge** is an open-world voxel RPG forked from [Veloren](https://veloren.net) — rebuilt to be LAN-first and authentication-free.
Play solo, host a game for friends on your local network, or run your own dedicated server — all **without an internet account or central authentication**.

> Built for people who want to just launch the game and play.

---

## What is Nova-Forge?

Nova-Forge is a fork of the open-source voxel RPG [Veloren](https://veloren.net). It strips out the mandatory online authentication layer and adds first-class LAN co-op hosting, singleplayer world management, and quality-of-life improvements for players who want a self-hosted or offline experience.

Nova-Forge stubs out the Veloren authentication layer so that every play mode — singleplayer, LAN game, dedicated server — works out of the box with no external dependencies.

---

## What makes Nova-Forge different?

| Feature | Nova-Forge | Upstream Veloren |
|---|---|---|
| No account required | ✅ | ❌ (online auth) |
| Singleplayer | ✅ | ✅ |
| LAN co-op hosting | ✅ (built-in) | manual setup |
| Dedicated server | ✅ (auth-free) | requires auth |
| Public server browser | optional | ✅ |

---

## Getting Started

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

## Play Modes

### Singleplayer
Launch the client, click **Singleplayer**, and you are in. No account, no server, no waiting.

### LAN Co-op
Click **Host LAN Game** to spin up a local server that friends on the same network can join immediately. No port-forwarding required for typical home networks.

### Dedicated Server
```bash
./nova-forge.sh server
```
Runs a fully self-contained server. Players connect with any username — no external authentication.

---

## Building from Source

```bash
# Dev build (fast iteration)
./nova-forge.sh build

# Optimised release build (also copies assets/ beside the binaries)
./nova-forge.sh release

# Launch the release client
./nova-forge.sh run-release

# Run tests
./nova-forge.sh test
```

> **Windows note:** After `./nova-forge.sh release`, the `assets/` folder is copied into
> `target/release/` so you can run `nova-forge-voxygen.exe` directly from that directory
> (e.g. by double-clicking). If you move the binary elsewhere, copy the `assets/` folder
> alongside it, or set the `NOVA_FORGE_ASSETS` environment variable to point to the assets
> directory in the repository root.

---

## Project Roadmap

See [`ROADMAP.md`](ROADMAP.md) for the full milestone plan, including planned features, in-progress work, and identified design gaps (e.g. the player housing system).

---

## Contributing

Issues and pull requests are welcome. Focus areas for Nova-Forge:

- LAN discovery and connection UX
- Auth-free server stability and compatibility
- Singleplayer world management
- Performance on lower-end hardware

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

### **Q:** The game crashes with "Asset directory not found" when I launch it.

**A:** The client needs the `assets/` folder to be next to the executable. If you built with
`./nova-forge.sh release`, run `./nova-forge.sh run-release` instead of launching the binary
directly, or copy the `assets/` folder from the repository root into the same directory as
`nova-forge-voxygen.exe`. Alternatively, set the `NOVA_FORGE_ASSETS` environment variable to the
full path of the repository's `assets/` directory before launching.

---

## Legal & Credits

### Fork origin

Nova-Forge is a **fork of [Veloren](https://veloren.net)**, an open-world voxel RPG developed by the Veloren contributors.
Veloren source code and project home: <https://gitlab.com/veloren/veloren>

Nova-Forge is **not affiliated with or endorsed by the Veloren project or its maintainers.**

### License

Nova-Forge inherits the **GNU General Public License v3.0** from Veloren.

- License file: [`LICENSE`](LICENSE)
- Full license text: <https://www.gnu.org/licenses/gpl-3.0-standalone.html>

Under the GPLv3:
- The source code must remain open and publicly available.
- Any redistribution of Nova-Forge (modified or unmodified) must include the full license text.
- Modified versions must also be released under the GPLv3.

### Credits — Veloren contributors

Nova-Forge exists because of the extraordinary work of the Veloren community:

- **Developers** — the engineers who built the voxel engine, ECS, networking, and tooling
- **Artists** — voxel modellers, texture artists, and UI designers
- **Composers & sound designers** — the musicians and audio engineers behind Veloren's soundtrack
- **Translators** — the community members who localised the game into dozens of languages

Full contributor list: <https://gitlab.com/veloren/veloren/-/graphs/master>

### Credits — Nova-Forge contributors

<!-- TODO: add Nova-Forge-specific contributors here as the project grows -->

### Third-party assets & libraries

<!-- TODO: list any third-party assets or libraries used specifically by Nova-Forge (beyond those already covered by Veloren's own acknowledgements) -->
