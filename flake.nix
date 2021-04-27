{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixCargoIntegration = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    let
      output = inputs.nixCargoIntegration.lib.makeOutputs {
        root = ./.;
        buildPlatform = "crate2nix";
        overrides = {
          build = common: prev: {
            runTests = !prev.release && prev.runTests;
          };
          common = prev:
            let
              pkgs = prev.pkgs;
              lib = pkgs.lib;

              gitLfsCheckFile = ./assets/voxygen/background/bg_main.png;
              utils = import ./nix/utils.nix { inherit pkgs; };

              sourceInfo =
                if inputs.self.sourceInfo ? rev
                then inputs.self.sourceInfo // {
                  # Tag would have to be set manually for stable releases flake
                  # because there's currently no way to get the tag via the interface.
                  # tag = v0.9.0;
                }
                else (throw "Can't get revision because the git tree is dirty");

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
                ln -sf ${./assets} $out/assets
              '';

              velorenOverride = common: oldAttr:
                if common.cargoPkg.name == "veloren-voxygen"
                then
                  {
                    VELOREN_USERDATA_STRATEGY = "system";
                    postInstall = ''
                      if [ -f $out/bin/veloren-voxygen ]; then
                        wrapProgram $out/bin/veloren-voxygen \
                          --set VELOREN_ASSETS ${veloren-assets} \
                          --set LD_LIBRARY_PATH ${lib.makeLibraryPath common.runtimeLibs}
                      fi
                    '';
                    patches = [
                      (import ./nix/nullOggPatch.nix { nullOgg = ./assets/voxygen/audio/null.ogg; inherit pkgs; })
                    ];
                  }
                else if common.cargoPkg.name == "veloren-server-cli"
                then
                  {
                    VELOREN_USERDATA_STRATEGY = "system";
                    postInstall = ''
                      if [ -f $out/bin/veloren-server-cli ]; then
                        wrapProgram $out/bin/veloren-server-cli --set VELOREN_ASSETS ${veloren-assets}
                      fi
                    '';
                  }
                else { };
            in
            {
              crateOverrides = prev.crateOverrides // {
                veloren-world = oldAttrs: {
                  crateBin = lib.filter (bin: bin.name != "chunk_compression_benchmarks") oldAttrs.crateBin;
                };
                veloren-client = oldAttrs: {
                  crateBin = lib.filter (bin: bin.name != "bot") oldAttrs.crateBin;
                };
                veloren-common = oldAttrs: {
                  # Disable `git-lfs` check here since we check it ourselves
                  # We have to include the command output here, otherwise Nix won't run it
                  DISABLE_GIT_LFS_CHECK = utils.isGitLfsSetup gitLfsCheckFile;
                  # Declare env values here so that `common/build.rs` sees them
                  NIX_GIT_HASH = prettyRev;
                  NIX_GIT_TAG = tag;
                  crateBin = lib.filter (bin: bin.name != "csv_export" && bin.name != "csv_import") oldAttrs.crateBin;
                };
              };
              overrides = prev.overrides // {
                mainBuild = velorenOverride;
              };
            };
        };
      };
    in
    output // {
      defaultApp = builtins.mapAttrs (_: apps: apps.veloren-voxygen) output.apps;
      defaultPackage = builtins.mapAttrs (_: packages: packages.veloren-voxygen) output.packages;
    };
}
