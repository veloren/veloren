{ common
, release ? true
, crateToBuild ? "veloren-voxygen"
, disableGitLfsCheck ? false
, sourceInfo ? { }
}:
with common;
let
  utils = import ./utils.nix { inherit pkgs; };

  meta = with pkgs.lib; {
    description = "Veloren is a multiplayer voxel RPG written in Rust.";
    longDescription = ''
      Veloren is a multiplayer voxel RPG written in Rust.
      It is inspired by games such as Cube World, Legend of Zelda: Breath of the Wild, Dwarf Fortress and Minecraft.
    '';
    homepage = "https://veloren.net";
    license = licenses.gpl3;
    maintainers = [ maintainers.yusdacra ];
    # TODO: Make this work on BSD and Mac OS
    platforms = platforms.linux;
  };

  prettyRev = with sourceInfo; builtins.substring 0 8 rev + "/" + utils.dateTimeFormat lastModified;

  tag = with sourceInfo;
    if sourceInfo ? tag
    then sourceInfo.tag
    else "";

  # If gitTag has a tag (meaning the commit we are on is a *release*), use
  # it as version, else: just use the prettified hash we have, if we don't
  # have it the build fails.
  # Must be in format f4987672/2020-12-10-12:00
  version =
    if tag != "" then tag
    else if prettyRev != "" then prettyRev
    else throw "Need a tag or pretty revision in order to determine version";

  veloren-assets = pkgs.runCommand "makeAssetsDir" { } ''
    mkdir $out
    ln -sf ${../assets} $out/assets
  '';

  velorenVoxygenDesktopFile = pkgs.makeDesktopItem rec {
    name = "veloren-voxygen";
    exec = name;
    icon = ../assets/voxygen/logo.ico;
    comment =
      "Official client for Veloren - the open-world, open-source multiplayer voxel RPG";
    desktopName = "Voxygen";
    genericName = "Veloren Client";
    categories = "Game;";
  };

  velorenOverride = oldAttr: with pkgs;
    if crateToBuild == "veloren-voxygen"
    then
      {
        pname = "veloren-voxygen";
        inherit version;

        nativeBuildInputs = (oldAttr.nativeBuildInputs or [ ]) ++ [ makeWrapper copyDesktopItems ];
        desktopItems = [ velorenVoxygenDesktopFile ];
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
            "This package includes the official client, Voxygen."
          '';
        };
      }
    else if crateToBuild == "veloren-server-cli"
    then
      {
        pname = "veloren-server-cli";
        inherit version;

        nativeBuildInputs = (oldAttr.nativeBuildInputs or [ ]) ++ [ makeWrapper ];
        postInstall = ''
          wrapProgram $out/bin/veloren-server-cli --set VELOREN_ASSETS ${veloren-assets}
        '';
        meta = meta // {
          longDescription = ''
            ${meta.longDescription}
            "This package includes the server CLI."
          '';
        };
      }
    else { };

  velorenPackage =
    pkgs.naersk.buildPackage {
      name = crateToBuild;
      root = ../.;
      targets = [ crateToBuild ];
      nativeBuildInputs = common.crateDeps.nativeBuildInputs;
      buildInputs = common.crateDeps.buildInputs;
      override = (_: {
        # Disable `git-lfs` check here since we check it ourselves
        # We have to include the command output here, otherwise Nix won't run it
        DISABLE_GIT_LFS_CHECK = utils.isGitLfsSetup common.gitLfsCheckFile;
        # Declare env values here so that `common/build.rs` sees them
        NIX_GIT_HASH = prettyRev;
        NIX_GIT_TAG = tag;
        VELOREN_USERDATA_STRATEGY = "system";
      });
      overrideMain = velorenOverride;
      inherit release;
    };
in
velorenPackage
