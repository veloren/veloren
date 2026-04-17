# Nix / NixOS

Nova-Forge ships a `flake.nix` for reproducible builds.

## Requirements

- Nix with Flakes enabled (≥ 2.4)
- `git-lfs` installed and initialised in the repo

## Quick start

```shell
# Enter the dev shell
nix develop

# Build release binaries
nix build .#nova-forge-voxygen
nix build .#nova-forge-server-cli
```

See the root `README.md` for general build instructions.
