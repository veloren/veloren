## Read this first!

#### You'll need at least Nix `2.4pre20210317_8a5203d`!

Since this repo uses a new Nix feature called "Flakes", it is recommended to enable it.
It massively improves the `nix` CLI UX, and adds many useful features.
We include instructions for Nix without flakes enabled, but using flakes is the recommended way.

See the [NixOS wiki](https://nixos.wiki/wiki/Flakes) for information on how to enable and use flakes.

It is recommended to first set up the [Cachix](https://cachix.org) cache to save time with builds:
```shell
nix shell nixpkgs#cachix -c cachix use veloren-nix
# or if you don't have flakes:
nix-shell -p cachix --run "cachix use veloren-nix"
```

As this repository uses `git-lfs`, please make sure `git-lfs` is in your path.
If you have a locally cloned repo, you can make sure it is set up with:
```shell
git lfs install --local && git lfs fetch && git lfs checkout
```
This should be automatically done if you use the development shell.

If you get an issue such as `WARN gfx_backend_vulkan: Unable to create Vulkan instance: VkError(ERROR_INCOMPATIBLE_DRIVER)`,
it might be that your system nixpkgs version and veloren repo nixpkgs version might be too far apart. In that case, you can try
changing your system nixpkgs to the unstable channel, or change the `nixpkgs` input in the `flake.nix` to match your system
nixpkgs.

## Usage for players

### With flakes

If you just want to run the game without installing it, you can do so with:
```shell
# Voxygen (the default):
nix run gitlab:veloren/veloren
# Server CLI:
nix run gitlab:veloren/veloren#veloren-server-cli
# or if you have a local repo
nix run
nix run .#veloren-server-cli
```

To install the game into your user profile:
```shell
# Voxygen:
nix profile install gitlab:veloren/veloren
# Server CLI:
nix profile install giltab:veloren/veloren#veloren-server-cli
# or if you have a local repo:
nix profile install
nix profile install .#veloren-server-cli
```

To install (for example) Voxygen on your system, the NixOS configuration (if you use a flake based setup) could look something like this:
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

### Without flakes

You can do this to run the game without installing it (you will need a local clone of the repo though):
```shell
# build the game
nix-build nix/default.nix
# run it
./result/bin/veloren-voxygen
# or for server cli
./result-2/bin/veloren-server-cli
```

To install Voxygen and server CLI into user profile:
```shell
nix-env -f nix/default.nix -i
```

### Notes for non-NixOS setups

You'll need to use [nixGL](https://github.com/guibou/nixGL) to be able to run the game (after installing it):
```shell
## For Intel and AMD:
# Install it (sadly no flake yet)
nix-env -f https://github.com/guibou/nixGL/archive/master.tar.gz -iA nixGLIntel
nixGLIntel veloren-voxygen
## For Nvidia:
# Install it
nix-env -f https://github.com/guibou/nixGL/archive/master.tar.gz -iA nixGLNvidia
nixGLNvidia veloren-voxygen
## For Nvidia driver on hybrid hardware:
# Install it
nix-env -f https://github.com/guibou/nixGL/archive/master.tar.gz -iA nixGLNvidiaBumblebee
nixGLNvidiaBumblebee veloren-voxygen
```

## Usage for developers

The development shell automatically setups the Cachix cache for you, so it is recommended to be in the dev shell always.
If you have the Cachix cache setup in `~/.config/nix/nix.conf` (as described in the beginning of this document), then this isn't a necessity.

### With flakes

To enter a shell environment with the necessary tools:
```shell
nix develop
```

You can use the `bundle` subcommand to bundle the game into a single distro-agnostic executable file:
```shell
## bundling latest commit to master
# Voxygen:
nix bundle gitlab:veloren/veloren
# Server CLI:
nix bundle gitlab:veloren/veloren#veloren-server-cli
## for local repo:
# Voxygen:
nix bundle .#veloren-voxygen
# Server CLI:
nix bundle .#veloren-server-cli
```

### Without flakes

To enter the development shell:
```shell
nix-shell nix/shell.nix
```

### Direnv

This only works if you have flakes. There is an issue with the git-lfs hook in `shellHook` erroring out with `use nix`, so you'll have to enable flakes if you want to use the `envrc` file included.

If you have [direnv](https://direnv.net) and [nix-direnv](https://github.com/nix-community/nix-direnv) on your system, you can copy the `envrc` file to the root of the repository as `.envrc`:
```shell
cp nix/envrc .envrc
```

## Managing dependencies

### With flakes

If a specific revision is specified in `flake.nix`, you will have to update that first, either by specifying a new desired revision or by removing it.

You can update dependencies with:
```shell
nix flake update
```

### Without flakes

It is inadvised to update revisions without the use of `nix flake update` as it's both tedious and error-prone to attempt setting all fields to their correct values in both `flake.nix` and `flake.lock`, but if you need to do it for testing, `flake.lock` is where legacy nix commands get the input revisions from (through `flake-compat`), regardless of what is specified in `flake.nix` (see https://github.com/edolstra/flake-compat/issues/10). 

Modify the relevant `rev` field in `flake.lock` to what you need - you can use `nix-prefetch-git` to find an up-to-date revision. Leave the `narHash` entry as is and attempt a rebuild to find out what its value should be.

## Formatting

Use [alejandra](https://github.com/kamadorueda/alejandra) to format files.

To format every Nix file:
```shell
# From repository root
alejandra .
```
