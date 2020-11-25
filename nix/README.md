### How to use

To build and install Voxygen and the server CLI into user profile, run:
```shell
nix-env -f nix/default.nix -i
```
You can configure what to install by changing the `cratesToBuild` argument:
```shell
nix-env -f nix/default.nix --arg cratesToBuild '["veloren-voxygen"]'
```
For example, this will install Voxygen only.

To enter the development shell (which includes all tools mentioned in this readme + tools you'll need to develop Veloren), run:
```shell
nix-shell nix/shell.nix
```

### Managing Cargo.nix

Enter the development shell.

To update `Cargo.nix` (and `crate-hashes.json`) using latest `Cargo.lock`, run:
```shell
crate2nix generate -f ../Cargo.toml
```

### Managing dependencies

We use [niv](https://github.com/nmattia/niv) to manage dependencies.

To update the dependencies, run (from repository root):
```shell
niv update
```

### Formatting

Use [nixfmt](https://github.com/serokell/nixfmt) to format files.

To format every Nix file in current working directory:
```shell
nixfmt *.nix
```
