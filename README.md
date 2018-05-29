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

2. compile crates
```bash
( cd worldgen && cargo build )
( cd worldsim && cargo build )
( cd worldtest && cargo build )
( cd server && cargo build )
( cd frontend && cargo build )
( cd client && cargo build )
```
3. run it
```bash
./frontend/target/debug/frontend
#the output "A simulation tick has occured." apears
```
