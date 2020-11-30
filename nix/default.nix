{
  # `crate2nix` doesn't support profiles in `Cargo.toml`, so default to release.
  # Otherwise bad performance (non-release is built with opt level 0)
  release ? true
, cratesToBuild ? [ "veloren-voxygen" "veloren-server-cli" ]
, system ? builtins.currentSystem
, nixpkgs ? sources.nixpkgs
, nvidia ? false
, sources ? import ./sources.nix { inherit system; }
}:
let
  common = import ./common.nix { inherit nixpkgs system sources; };
  inherit (common) pkgs;

  meta = with pkgs.stdenv.lib; {
    description = "Veloren is a multiplayer voxel RPG written in Rust.";
    longDescription = ''
      Veloren is a multiplayer voxel RPG written in Rust.
      It is inspired by games such as Cube World, Legend of Zelda: Breath of the Wild, Dwarf Fortress and Minecraft.
    '';
    homepage = "https://veloren.net";
    upstream = "https://gitlab.com/veloren/veloren";
    license = licenses.gpl3;
    maintainers = [ maintainers.yusdacra ];
    platforms = platforms.all;
  };

  isGitLfsSetup =
    let
      gitLfsCheckOutput = with common;
        builtins.readFile (pkgs.runCommand "gitLfsCheck" { } ''
          [ "$(${pkgs.file}/bin/file --mime-type ${gitLfsCheckFile})" = "${gitLfsCheckFile}: image/png" ]
          printf $? > $out
        '');
    in
    if gitLfsCheckOutput == "0" then
      true
    else
      abort ''
        Git Large File Storage (`git-lfs`) has not been set up correctly.
        Most common reasons:
          - `git-lfs` was not installed before cloning this repository.
          - This repository was not cloned from the primary GitLab mirror.
          - The GitHub mirror does not support LFS.
        See the book at https://book.veloren.net/ for details.
      '';

  makeGitCommand = subcommands: name:
    builtins.readFile (pkgs.runCommand name { } ''
      cd ${
      # Only copy the `.git` directory to nix store, anything else is a waste.
        builtins.path {
          path = ../.git;
          # Nix store path names don't accept names that start with a dot.
          name = "veloren-git-dir";
        }
      }
      (${pkgs.git}/bin/git ${subcommands}) > $out
    '');

  gitHash = makeGitCommand
    "log -n 1 --pretty=format:%h/%cd --date=format:%Y-%m-%d-%H:%M --abbrev=8"
    "getGitHash";

  gitTag =
    # If the git command errors out we feed an empty string
    makeGitCommand "describe --exact-match --tags HEAD || printf ''"
      "getGitTag";

  # If gitTag has a tag (meaning the commit we are on is a *release*), use it as version
  # If not, we just use the prettified hash we have
  version = if gitTag != "" then gitTag else gitHash;
  # Sanitize version string since it contains not allowed characters for a Nix store path
  # Only used in the package name
  sanitizedVersion = pkgs.stdenv.lib.strings.sanitizeDerivationName version;

  veloren-assets = pkgs.runCommand "makeAssetsDir" { } ''
    mkdir $out
    ln -sf ${../assets} $out/assets
  '';

  veloren-crates = with pkgs;
    callPackage ./Cargo.nix {
      defaultCrateOverrides = with common;
        defaultCrateOverrides // {
          libudev-sys = _: {
            inherit (crateDeps.libudev-sys) nativeBuildInputs buildInputs;
          };
          alsa-sys = _: {
            inherit (crateDeps.alsa-sys) nativeBuildInputs buildInputs;
          };
          veloren-common = _: {
            # Disable `git-lfs` check here since we check it ourselves
            DISABLE_GIT_LFS_CHECK = isGitLfsSetup;
            # Declare env values here so that `common/build.rs` sees them
            NIX_GIT_HASH = gitHash;
            NIX_GIT_TAG = gitTag;
          };
          veloren-network = _: {
            inherit (crateDeps.veloren-network) nativeBuildInputs buildInputs;
          };
          veloren-server-cli = _: {
            name = "veloren-server-cli_${sanitizedVersion}";
            inherit version;
            VELOREN_USERDATA_STRATEGY = "system";
            nativeBuildInputs = [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/veloren-server-cli --set VELOREN_ASSETS ${veloren-assets}
            '';
            meta = meta // {
              longDescription = ''
                ${meta.longDescription}
                "This package includes the server CLI."
              '';
            };
          };
          veloren-voxygen = _: {
            name = "veloren-voxygen_${sanitizedVersion}";
            inherit version;
            VELOREN_USERDATA_STRATEGY = "system";
            inherit (crateDeps.veloren-voxygen) buildInputs;
            nativeBuildInputs = crateDeps.veloren-voxygen.nativeBuildInputs
            ++ [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/veloren-voxygen\
                --set VELOREN_ASSETS ${veloren-assets}\
                --set LD_LIBRARY_PATH ${
                  lib.makeLibraryPath common.voxygenNeededLibs
                }
            '';
            meta = meta // {
              longDescription = ''
                ${meta.longDescription}
                "This package includes the client, Voxygen."
              '';
            };
          };
        };
      inherit release pkgs;
    };

  makePkg = name: veloren-crates.workspaceMembers."${name}".build;
in
(pkgs.lib.genAttrs cratesToBuild makePkg)
