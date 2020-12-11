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

#### Using the flake

Due to the nature of flakes' reliance on git and the way `git-lfs` is configured for this repo, you must already have `git-lfs` in your environment when running nix commands on a local checkout. Run this to enter a shell environment with `git-lfs` in your path:
```shell
nix shell nixpkgs#git-lfs
```

To enter a shell environment with the necessary tools:
```shell
nix develop
```

If you simply want to run the latest version without necessarily installing it, you can do so with
```shell
# Voxygen (the default):
nix run gitlab:veloren/veloren
# Server CLI:
nix run gitlab:veloren/veloren#veloren-server-cli
```

To install (for example) the game client on your system, the configuration could look something like this:
```nix
{ description = "NixOS configuration with flakes";

  inputs.veloren.url = gitlab:veloren/veloren;

  outputs = { self, nixpkgs, veloren }: {
    nixosConfigurations.<your-hostname> = nixpkgs.lib.nixosSystem rec {
      system = <your-system-arch>;
      # ...
      modules = [
        # add to your overlay so that the packages appear in pkgs
        # for subsequent modules
        ({...}: {
          nixpkgs.overlays = [
            # ...
            (final: prev: {
              inherit (veloren.packages."${system}") veloren-voxygen;
            })
          ];

          # You can also add the flake to your registry
          nix.registry.veloren.flake = veloren;
          # with this, you can run latest master
          # regardless of version installed like this:
          # nix run veloren/master
        })

        # some module
        ({ pkgs, ... }: {
          environment.systemPackages = [
            pkgs.veloren-voxygen
          ];
        })
        # ...
      ];
    };
  };
}
```

### Managing Cargo.nix

Enter the development shell.

To update `Cargo.nix` (and `crate-hashes.json`) using latest `Cargo.lock`, run:
```shell
crate2nix generate -f ../Cargo.toml
```

### Managing dependencies

#### Nix with flakes enabled

If a specific revision is specified in `flake.nix`, you will have to update that first, either by specifying a new desired revision or by removing it.

You can update the dependencies individually or all at once from the root of the project:
```shell
# only nixpkgs
nix flake update --update-input nixpkgs
# everything
nix flake update --recreate-lock-file
```

See the [NixOS wiki](https://nixos.wiki/wiki/Flakes) for more information on how to use flakes.

#### Legacy nix

It is inadvised to update revisions without the use of `nix flake update` as it's both tedious and error-prone to attempt setting all fields to their correct values in both `flake.nix` and `flake.lock`, but if you need to do it for testing, `flake.lock` is where legacy nix commands get the input revisions from (through `flake-compat`), regardless of what is specified in `flake.nix` (see https://github.com/edolstra/flake-compat/issues/10). 

Modify the relevant `rev` field in `flake.lock` to what you need - you can use `nix-prefetch-git` to find an up-to-date revision. Leave the `narHash` entry as is and attempt a rebuild to find out what its value should be.

### Formatting

Use [nixpkgs-fmt](https://github.com/nix-community/nixpkgs-fmt) to format files.

To format every Nix file:
```shell
nixpkgs-fmt flake.nix nix/*.nix
```
