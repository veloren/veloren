{
/* `crate2nix` doesn't support profiles in `Cargo.toml`, so default to release.
   Otherwise bad performance (non-release is built with opt level 0)
*/
release ? true, cratesToBuild ? [ "veloren-voxygen" "veloren-server-cli" ]
, system ? builtins.currentSystem, nixpkgs ? sources.nixpkgs
, sources ? import ./sources.nix { inherit system; } }:

let
  common = import ./common.nix {
    inherit nixpkgs system;
    inherit (sources) nixpkgsMoz;
  };
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

  isGitLfsSetup = let
    checkFile = ../assets/voxygen/background/bg_main.png;
    gitLfsCheckOutput = builtins.readFile (pkgs.runCommand "gitLfsCheck" { } ''
      [ "$(${pkgs.file}/bin/file --mime-type ${checkFile})" = "${checkFile}: image/png" ]
      printf $? > $out
    '');
  in if gitLfsCheckOutput == "0" then
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

  veloren-assets = pkgs.runCommand "makeAssetsDir" { } ''
    mkdir $out
    ln -sf ${../assets} $out/assets
  '';

  veloren-crates = with pkgs;
    callPackage ./Cargo.nix {
      defaultCrateOverrides = with common;
        defaultCrateOverrides // {
          libudev-sys = _: { buildInputs = crateDeps.libudev-sys; };
          alsa-sys = _: { buildInputs = crateDeps.alsa-sys; };
          veloren-common = _: {
            # Disable `git-lfs` check here since we check it ourselves
            DISABLE_GIT_LFS_CHECK = isGitLfsSetup;
            # Declare env values here so that `common/build.rs` sees them
            NIX_GIT_HASH = gitHash;
            NIX_GIT_TAG = gitTag;
          };
          veloren-network = _: { buildInputs = crateDeps.veloren-network; };
          veloren-server-cli = _: {
            VELOREN_USERDATA_STRATEGY = "system";
            nativeBuildInputs = [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/veloren-server-cli --set VELOREN_ASSETS ${veloren-assets}
            '';
          };
          veloren-voxygen = _: {
            VELOREN_USERDATA_STRATEGY = "system";
            buildInputs = crateDeps.veloren-voxygen;
            nativeBuildInputs = [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/veloren-voxygen\
                --set LD_LIBRARY_PATH ${neededLibPathsVoxygen}\
                --set VELOREN_ASSETS ${veloren-assets}
            '';
          };
        };
      inherit release pkgs;
    };

  makePkg = name:
    pkgs.symlinkJoin {
      inherit version;
      name = "${name}_${version}";
      paths = [ veloren-crates.workspaceMembers."${name}".build ];
      meta = meta // {
        longDescription = ''
          ${meta.longDescription}
          ${if name == "veloren-voxygen" then
            "This package includes the client, Voxygen."
          else
            ""}
          ${if name == "veloren-server-cli" then
            "This package includes the server CLI."
          else
            ""}
        '';
      };
    };
in (pkgs.lib.genAttrs cratesToBuild makePkg)
