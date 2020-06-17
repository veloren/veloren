{ crateName ? "veloren-voxygen",
# `crate2nix` doesn't support profiles in `Cargo.toml`, so default to release. Otherwise bad performance (non-release is built with opt level 0)
release ? true, sources, nixpkgsSrc }:

let
  # Check if git-lfs is working.
  isGitLfsSetup =
    if builtins.pathExists ../assets/voxygen/background/bg_main.png then
      true
    else
      abort ''
        Git Large File Storage (git-lfs) has not been set up correctly.
        Most common reasons:
        	- git-lfs was not installed before cloning this repository
        	- this repository was not cloned from the primary gitlab mirror.
        	- The github mirror does not support lfs.
        See the book at https://book.veloren.net/ for details.
      '';

  pkgs = import ./nixpkgs.nix { inherit sources nixpkgsSrc; };

  # Only copy the `.git` directory to nix store, anything else is a waste.
  gitSrc = builtins.path {
    path = ../.git;
    name = "git";
  };
  gitHash = builtins.readFile (with pkgs;
    runCommand "getGitHash" { nativeBuildInputs = [ git ]; } ''
      cd ${gitSrc}
      git log -n 1 --pretty=format:%h/%cd --date=format:%Y-%m-%d-%H:%M --abbrev=8 > $out
    '');

  veloren = with pkgs;
    callPackage ./Cargo.nix {
      defaultCrateOverrides = defaultCrateOverrides // {
        libudev-sys = attrs: { buildInputs = [ pkg-config libudev ]; };
        alsa-sys = attrs: { buildInputs = [ pkg-config alsaLib ]; };
        veloren-common = attrs: {
          NIX_GIT_HASH = gitHash;
          # We need to include the result here otherwise nix won't evaluate the check.
          GIT_LFS_SETUP = isGitLfsSetup;
        };
        veloren-network = attrs: { buildInputs = [ pkg-config openssl ]; };
        veloren-voxygen = attrs: {
          buildInputs = [ atk cairo glib gtk3 pango ];
        };
      };
      inherit release pkgs;
    };
in veloren.workspaceMembers."${crateName}".build
