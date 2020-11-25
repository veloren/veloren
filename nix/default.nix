{
/* `crate2nix` doesn't support profiles in `Cargo.toml`, so default to release.
   Otherwise bad performance (non-release is built with opt level 0)
*/
release ? true, cratesToBuild ? [ "veloren-voxygen" "veloren-server-cli" ]
, system ? builtins.currentSystem
, sources ? import ./sources.nix { inherit system; } }:

let
  isBuildingCrate = name:
    builtins.any (otherName: name == otherName) cratesToBuild;
  isBuildingVoxygen = isBuildingCrate "veloren-voxygen";
  isBuildingServerCli = isBuildingCrate "veloren-server-cli";

  pkgs = import ./nixpkgs.nix { inherit sources system; };
  common = import ./common.nix { inherit pkgs; };

  meta = with pkgs; {
    description = "Veloren is a multiplayer voxel RPG written in Rust.";
    longDescription = ''
      Veloren is a multiplayer voxel RPG written in Rust.
      It is inspired by games such as Cube World, Legend of Zelda: Breath of the Wild, Dwarf Fortress and Minecraft.
    '';
    homepage = "https://veloren.net";
    upstream = "https://gitlab.com/veloren/veloren";
    license = lib.licenses.gpl3;
    maintainers = [ lib.maintainers.yusdacra ];
    platforms = lib.platforms.all;
  };

  makeGitCommand = subcommands: name:
    # Check if git-lfs is working. This is a partial check only,
    # the actual check is done in `common/build.rs`. We do this
    # so that the build fails early.
    if builtins.pathExists ../assets/voxygen/background/bg_main.png then
      builtins.readFile (pkgs.runCommand name { } ''
        cd ${
        # Only copy the `.git` directory to nix store, anything else is a waste.
          builtins.path {
            path = ../.git;
            # Nix store path names don't accept names that start with a dot.
            name = "git";
          }
        }
        ${pkgs.git}/bin/git ${subcommands} > $out
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

  gitHash = makeGitCommand
    "log -n 1 --pretty=format:%h/%cd --date=format:%Y-%m-%d-%H:%M --abbrev=8"
    "getGitHash";

  gitTag =
    # If the git command errors out we feed an empty string
    makeGitCommand "describe --exact-match --tags HEAD || printf ''"
    "getGitTag";

  version = if gitTag != "" then gitTag else gitHash;

  veloren-crates = with pkgs;
    callPackage ./Cargo.nix {
      defaultCrateOverrides = with common;
        defaultCrateOverrides // {
          libudev-sys = _: { buildInputs = crateDeps.libudev-sys; };
          alsa-sys = _: { buildInputs = crateDeps.alsa-sys; };
          veloren-common = _:
            (if isBuildingServerCli then {
              DISABLE_GIT_LFS_CHECK = true;
            } else
              { }) // {
                # Declare env values here so that `common/build.rs` sees them
                NIX_GIT_HASH = gitHash;
                NIX_GIT_TAG = gitTag;
              };
          veloren-network = _: { buildInputs = crateDeps.veloren-network; };
          veloren-server-cli = _: { VELOREN_USERDATA_STRATEGY = "system"; };
          veloren-voxygen = _: {
            VELOREN_USERDATA_STRATEGY = "system";
            buildInputs = crateDeps.veloren-voxygen;
            nativeBuildInputs = [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/veloren-voxygen --set LD_LIBRARY_PATH ${neededLibPaths}
            '';
          };
        };
      inherit release pkgs;
    };

  veloren-assets = pkgs.symlinkJoin {
    inherit version;
    name = "veloren-assets_${version}";
    paths = [
      (pkgs.runCommand "mkVelorenAssetsDir" { } ''
        mkdir -p $out/share/veloren
        ln -sf ${../assets} $out/share/veloren/assets
      '')
    ];
    meta = meta // {
      longDescription = ''
        ${meta.longDescription}
        This package includes the assets.
      '';
    };
  };

  makePkg = name:
    pkgs.symlinkJoin {
      inherit version;
      name = "${name}_${version}";
      paths = [ veloren-crates.workspaceMembers."${name}".build ];
      meta = meta // {
        longDescription = ''
          ${meta.longDescription}
          ${if isBuildingVoxygen then
            "This package includes the client, Voxygen."
          else
            ""}
          ${if isBuildingServerCli then
            "This package includes the server CLI."
          else
            ""}
        '';
      };
    };
in (pkgs.lib.genAttrs cratesToBuild makePkg) // { inherit veloren-assets; }
