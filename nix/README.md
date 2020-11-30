## Important

If you are going to call the derivations with a custom `nixpkgs` argument, make sure that the `nixpkgs` you pass is on at least the same commit or newer than it.
Unexpected errors may pop up if you use an older version. Same goes for the `sources` argument.

### How to use

To enter the development shell (which includes all tools mentioned in this readme + tools you'll need to develop Veloren), run:
```shell
nix-shell nix/shell.nix
```
It is recommended that you enter the dev shell before starting to build using `nix-build` or `nix-env` (anything which build stuff),
since it will setup a Cachix cache for you. (you can configure this for your user's `nix.conf` by running `cachix use veloren-nix` once in the dev shell,
which will make the cache available when you run commands outside of the dev shell).

If you have [direnv](https://direnv.net) setup on your system, it is also recommended to copy the `envrc`
(or `envrc-nvidia`, if you have an Nvidia GPU) file to the root of the repository as `.envrc`:
```shell
cp nix/envrc .envrc
```
This will make your env have the dev env setup automatically.

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

If you aren't on NixOS, you can run `veloren-voxygen` using the provided `nixGLIntel` in the dev shell:
```shell
nixGLIntel veloren-voxygen
```
If you have an Nvidia GPU, you can enter the dev shell like so:
```shell
nix-shell nix/shell.nix --arg nvidia true
```
And you'll be able to use `nixGLNvidia` and `nixGLNvidiaBumblebee`.

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

Use [nixpkgs-fmt](https://github.com/nix-community/nixpkgs-fmt) to format files.

To format every Nix file:
```shell
nixpkgs-fmt nix/*.nix
```
