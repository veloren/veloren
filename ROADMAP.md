# Nova-Forge Roadmap

This document tracks the planned development of Nova-Forge across milestone phases.
Checked items are complete (or substantially complete); unchecked items are planned or in progress.

---

## Phase 0 — Foundation (Completed / Ongoing)

- [x] Fork Veloren, rename binaries to `nova-forge-voxygen` / `nova-forge-server-cli`
- [x] Strip mandatory authentication (stub auth layer, allow any username)
- [x] LAN discovery and auto-connect
- [x] `nova-forge.sh` build/run/release helper script
- [x] Nix flake for reproducible builds
- [ ] Persistent singleplayer world save management UI *(in progress)*

---

## Phase 1 — Core Nova-Forge Modifications

Items that modify or extend existing base engine systems.

### GUI Scale slider for large/HiDPI monitors *(partially complete)*

The current absolute-scale slider was capped at 2.0×, making icons and text very small on 4K or ultrawide monitors.

- [x] Extend slider range from 2.0× to 4.0× for large monitor users
- [x] Update the dropdown preset list to include 2.5, 3.0, 3.5, 4.0
- [ ] Add a "DPI-aware auto" default that detects monitor DPI and sets a sensible scale *(PLANNED)*

### Singleplayer world management *(PLANNED)*

- List, create, rename, and delete singleplayer worlds from the main menu
- Per-world settings (seed, day length, difficulty)

### LAN server UX improvements *(PARTIALLY COMPLETE)*

- [x] Show LAN server version and player count in the browser list
- [ ] Connection status feedback during discovery

### Settings persistence *(PLANNED)*

- Ensure all Nova-Forge-specific settings (GUI scale, LAN preferences) are saved to the user profile

---

## Phase 2 — Player Housing System

**Status: Design Phase — gaps identified below**

Player housing is a major new system. The following design gaps must be resolved before implementation begins.

### Design Gaps

1. **Plot ownership model** — How are plots claimed? (proximity claim, purchase with in-game currency, server-admin grant?) Need to define the claim radius, max plots per player, and what happens when a player leaves the server permanently.
2. **Plot boundaries** — Fixed-size rectangular chunks vs. flexible polygon plots? How are boundaries visualised in-world (fences, markers, highlighted voxels)?
3. **Persistence backend** — Houses are voxel structures. Where are they stored? (server-side SQLite extension, separate file per plot, embedded in world chunks?) Need a schema for plot metadata (owner UUID, position, size, permissions).
4. **Build permissions system** — Who can place/break blocks inside a plot? (owner only, trusted list, guild members, visitors?) Need a permission enum and a UI for managing the list.
5. **Furniture / decoration system** — Are furnishings separate entity-objects or placed voxels? If entities, new ECS components (`Furniture`, `PlotObject`) are needed. If voxels, a mechanism to distinguish "owned structure" voxels from world terrain is required.
6. **Economy integration** — Is there an in-game currency for purchasing plots or furnishings? Nova-Forge currently has no economy layer — decision needed: implement a basic coin system or keep housing free.
7. **Server configuration** — Server admins need `settings.ron` options: enable/disable housing, max plot size, max plots per player, housing zone bounds.
8. **World zones** — Where can players build? (anywhere, designated town districts, purchasable wilderness plots?) Define zone types and how they are marked on the map.
9. **Migration / import** — Should players be able to import voxel blueprints (`.vox` files) as house templates?
10. **Multiplayer sync** — Real-time block-place events must be validated server-side and broadcast to nearby clients. Define the network message types needed.

### Implementation tasks (once gaps are resolved)

- `PlotClaim` server-side component and storage
- `/claim`, `/unclaim`, `/trust`, `/untrust` chat commands
- Plot boundary visualisation (highlight shader pass or client-side voxel overlay)
- Build-mode UI (toggle, block palette, undo/redo)
- Furniture entity type + placement UI
- Housing tab in the map window showing owned plots
- Server admin panel entries for housing config

---

## Phase 3 — Gameplay Extensions

- Custom skill trees / talent system (Nova-Forge exclusive skills)
- Seasonal events calendar
- Extended crafting: player-made blueprints

---

## Phase 4 — Polish & Release

- Installer / launcher (Windows `.msi`, Linux AppImage, macOS `.dmg`)
- Auto-update mechanism for the launcher
- Public server listing (opt-in)
- Full localisation pass for all Nova-Forge-specific UI strings
