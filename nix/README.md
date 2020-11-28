### How to use

To enter the development shell (which includes all tools mentioned in this readme + tools you'll need to develop Veloren), run:
```shell
nix-shell nix/shell.nix
```
It is recommended that you enter the dev shell before starting to build using `nix-build` or `nix-env` (anything which build stuff),
since it will setup a Cachix cache for you. (you can configure this for your user's `nix.conf` by running `cachix use veloren-nix` once in the dev shell,
which will make the cache available when you run commands outside of the dev shell).

To build and install Voxygen and the server CLI into user profile, run:
```shell
nix-env -f nix/default.nix -i
```
You can configure what to install by changing the `cratesToBuild` argument:
```shell
nix-env -f nix/default.nix --arg cratesToBuild '["veloren-voxygen"]'
```
For example, this will install Voxygen only.

You can configure the crates to be built with debug mode (not recommended, equals to `opt-level = 0`):
```shell
nix-env -f nix/default.nix --arg release false
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
