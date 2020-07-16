{ crateName ? "veloren-voxygen",
# `crate2nix` doesn't support profiles in `Cargo.toml`, so default to release. Otherwise bad performance (non-release is built with opt level 0)
release ? true, nixpkgs ? <nixpkgs>, system ? builtins.currentSystem
, sources ? import ./sources.nix { inherit system; } }:

let
  gitHash =
    # Check if git-lfs is working.
    if builtins.pathExists ../assets/voxygen/background/bg_main.png then
      builtins.readFile (pkgs.runCommand "getGitHash" { } ''
        cd ${
        # Only copy the `.git` directory to nix store, anything else is a waste.
          builtins.path {
            path = ../.git;
            # Nix store path names don't accept names that start with a dot.
            name = "git";
          }
        }
        ${pkgs.git}/bin/git log -n 1 --pretty=format:%h/%cd --date=format:%Y-%m-%d-%H:%M --abbrev=8 > $out
      '')
    else
      abort ''
        Git Large File Storage (git-lfs) has not been set up correctly.
        Most common reasons:
        	- git-lfs was not installed before cloning this repository
        	- this repository was not cloned from the primary gitlab mirror.
        	- The github mirror does not support lfs.
        See the book at https://book.veloren.net/ for details.
      '';

  pkgs = import ./nixpkgs.nix { inherit sources nixpkgs system; };

  veloren = with pkgs;
    callPackage ./Cargo.nix {
      defaultCrateOverrides = defaultCrateOverrides // {
        libudev-sys = _: { buildInputs = [ pkg-config libudev ]; };
        alsa-sys = _: { buildInputs = [ pkg-config alsaLib ]; };
        veloren-common = _: { NIX_GIT_HASH = gitHash; };
        veloren-network = _: { buildInputs = [ pkg-config openssl ]; };
        veloren-voxygen = _: { buildInputs = [ atk cairo glib gtk3 pango ]; };
      };
      inherit release pkgs nixpkgs;
    };
in veloren.workspaceMembers."${crateName}".build
