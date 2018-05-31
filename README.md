# Welcome to Verloren

[![Build Status](https://travis-ci.org/veloren/game.svg?branch=master)](https://travis-ci.org/veloren/game)

## What is Verloren?
Verloren is a multiplayer voxel game inspired by Cube World. It aims to emulate the feel of Cube World while deviating in its features.

## Licensing and Contribution

Verloren is an open-source community project licensed under the General Public License version 3. We gratefully welcome community contributions, both technical and editorial.

## Compilation

1. Install dependencies necessary for building

```bash
# (Arch Linux):
pacman -S rust
pacman -U https://archive.archlinux.org/packages/s/sfml/sfml-2.4.2-5-x86_64.pkg.tar.xz #needed for now, because sfml is normaly 2.5 and csfml only 2.4
pacman -S csfml
```

2. Compile and run `worldtest`

```bash
(cd worldtest && cargo run)
```

3. Compile and run `server-cli`

```bash
(cd server-cli && cargo run)
```

4. Compile and run `frontend`

```bash
(cd frontend && cargo run)
```
