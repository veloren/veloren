# Welcome to veloren

## What is veloren
veloren is a multiplayer voxelbased game which is heavily influenced by cubeworld.

## how to compile it
1. install dependencies
```bash
#arch:
pacman -S rust
pacman -U https://archive.archlinux.org/packages/s/sfml/sfml-2.4.2-5-x86_64.pkg.tar.xz #needed for now, because sfml is normaly 2.5 and csfml only 2.4
pacman -S csfml
```

2. compile and run tests
```bash
( cd worldtest && cargo run )
```

3. compile and run server
```bash
( cd server-cli && cargo run )
```

4. compile and run client
```bash
( cd frontend && cargo run )
```
