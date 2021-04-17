{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    naersk = {
      url = "github:yusdacra/naersk/veloren";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixCargoIntegration = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.naersk.follows = "naersk";
    };
  };

  outputs = inputs:
    let
      output = inputs.nixCargoIntegration.lib.makeOutputs {
        root = ./.;
        overrides = {
          common = prev: {
            gitLfsCheckFile = ./assets/voxygen/background/bg_main.png;
            utils = import ./nix/utils.nix { pkgs = prev.pkgs; };
          };
          build = common: prevb:
            let
              pkgs = common.pkgs;
              sourceInfo =
                if inputs.self.sourceInfo ? rev
                then inputs.self.sourceInfo // {
                  # Tag would have to be set manually for stable releases flake
                  # because there's currently no way to get the tag via the interface.
                  # tag = v0.9.0;
                }
                else (throw "Can't get revision because the git tree is dirty");

              prettyRev = with sourceInfo; builtins.substring 0 8 rev + "/" + common.utils.dateTimeFormat lastModified;

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
                ln -sf ${./assets} $out/assets
              '';

              velorenOverride = oldAttr:
                if common.cargoPkg.name == "veloren-voxygen"
                then
                  {
                    nativeBuildInputs = oldAttr.nativeBuildInputs ++ [ pkgs.makeWrapper ];
                    postInstall = ''
                      wrapProgram $out/bin/veloren-voxygen\
                        --set VELOREN_ASSETS ${veloren-assets}\
                        --set LD_LIBRARY_PATH ${
                          pkgs.lib.makeLibraryPath common.runtimeLibs
                        }
                    '';
                  }
                else if common.cargoPkg.name == "veloren-server-cli"
                then
                  {
                    nativeBuildInputs = oldAttr.nativeBuildInputs ++ [ pkgs.makeWrapper ];
                    postInstall = ''
                      wrapProgram $out/bin/veloren-server-cli --set VELOREN_ASSETS ${veloren-assets}
                    '';
                  }
                else { };
            in
            {
              allRefs = true;
              override = old: (prevb.override old) // {
                # Disable `git-lfs` check here since we check it ourselves
                # We have to include the command output here, otherwise Nix won't run it
                DISABLE_GIT_LFS_CHECK = common.utils.isGitLfsSetup common.gitLfsCheckFile;
                # Declare env values here so that `common/build.rs` sees them
                NIX_GIT_HASH = prettyRev;
                NIX_GIT_TAG = tag;
                VELOREN_USERDATA_STRATEGY = "system";
              };
              overrideMain = old: (prevb.overrideMain old) // (velorenOverride old);
            };
        };
      };
    in
    output // {
      defaultApp = builtins.mapAttrs (_: apps: apps.veloren-voxygen-debug) output.apps;
      defaultPackage = builtins.mapAttrs (_: packages: packages.veloren-voxygen-debug) output.packages;
    };
}
