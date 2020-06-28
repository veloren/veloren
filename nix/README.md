### How to use

To build Voxygen, run:
`nix build`

To build another binary, run:
`nix build --arg crateName "<binary name here>"`

To enter the development shell (which includes all tools mentioned in this readme), run:
`nix-shell shell.nix`

### Managing Cargo.nix

Enter the development shell.

To update `Cargo.nix` (and `crate-hashes.json`) using latest `Cargo.lock`, run:
`crate2nix generate -f ../Cargo.toml`

### Managing dependencies

We use [niv](https://github.com/nmattia/niv) to manage dependencies.

Enter the development shell in repository root:
`cd .. && nix-shell nix/shell.nix`

To update the dependencies, run:
`niv update`

### Formatting

Use [nixfmt](https://github.com/serokell/nixfmt) to format files.

To format every file:
`nixfmt *.nix`
