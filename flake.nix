{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs.nci.url = "github:yusdacra/nix-cargo-integration";

  outputs = inputs:
    inputs.nci.lib.makeOutputs {
      root = ./.;
      defaultOutputs = {
        package = "veloren-voxygen";
        app = "veloren-voxygen";
      };
      perCrateOverrides = {
        veloren-voxygen = {
          packageMetadata = _: {
            features = rec {
              release = ["default-publish"];
              debug = release;
              test = release;
            };
          };
        };
      };
      overrides = {
        cCompiler = common: {
          cCompiler = common.pkgs.clang;
          useCCompilerBintools = true;
        };
        crates = common: prev: let
          pkgs = common.pkgs;
          lib = pkgs.lib;

          gitLfsCheckFile = ./assets/voxygen/background/bg_main.jpg;
          utils = import ./nix/utils.nix {inherit pkgs;};

          sourceInfo =
            if inputs.self.sourceInfo ? rev
            then
              inputs.self.sourceInfo
              // {
                # Tag would have to be set manually for stable releases flake
                # because there's currently no way to get the tag via the interface.
                # tag = v0.9.0;
              }
            else (throw "Can't get revision because the git tree is dirty");

          prettyRev = with sourceInfo; builtins.substring 0 8 rev + "/" + utils.dateTimeFormat lastModified;

          tag =
            if sourceInfo ? tag
            then sourceInfo.tag
            else "";

          # If gitTag has a tag (meaning the commit we are on is a *release*), use
          # it as version, else: just use the prettified hash we have, if we don't
          # have it the build fails.
          # Must be in format f4987672/2020-12-10-12:00
          version =
            if tag != ""
            then tag
            else if prettyRev != ""
            then prettyRev
            else throw "Need a tag or pretty revision in order to determine version";

          veloren-assets = pkgs.runCommand "makeAssetsDir" {} ''
            mkdir $out
            ln -sf ${./assets} $out/assets
          '';

          configMoldLinker = ''
            cat >>$CARGO_HOME/config.toml <<EOF
              [target.x86_64-unknown-linux-gnu]
              linker = "clang"
              rustflags = ["-C", "link-arg=-fuse-ld=${lib.getExe pkgs.mold}"]
            EOF
          '';
        in {
          veloren-common = oldAttrs: {
            # Disable `git-lfs` check here since we check it ourselves
            # We have to include the command output here, otherwise Nix won't run it
            DISABLE_GIT_LFS_CHECK = utils.isGitLfsSetup gitLfsCheckFile;
            # Declare env values here so that `common/build.rs` sees them
            NIX_GIT_HASH = prettyRev;
            NIX_GIT_TAG = tag;
          };
          veloren-voxygen-deps = oldAttrs: {
            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
          };
          veloren-voxygen = oldAttrs: {
            inherit version;

            buildInputs =
              (oldAttrs.buildInputs or [])
              ++ (
                with pkgs; [
                  alsa-lib
                  libxkbcommon
                  udev
                  xorg.libxcb
                ]
              );
            nativeBuildInputs =
              (oldAttrs.nativeBuildInputs or [])
              ++ (with pkgs; [python3 makeWrapper]);

            VELOREN_USERDATA_STRATEGY = "system";
            SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";

            dontUseCmakeConfigure = true;
            doCheck = false;

            preConfigure = ''
              ${oldAttrs.preConfigure or ""}
              substituteInPlace voxygen/src/audio/soundcache.rs \
                --replace \
                "../../../assets/voxygen/audio/null.ogg" \
                "${./assets/voxygen/audio/null.ogg}"
            '';
            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
            postInstall = ''
              ${oldAttrs.postInstall or ""}
              if [ -f $out/bin/veloren-voxygen ]; then
                wrapProgram $out/bin/veloren-voxygen \
                  --set VELOREN_ASSETS ${veloren-assets} \
                  --set LD_LIBRARY_PATH ${lib.makeLibraryPath common.runtimeLibs}
              fi
            '';
          };
          veloren-server-cli = oldAttrs: {
            nativeBuildInputs = (oldAttrs.nativeBuildInputs or []) ++ [pkgs.makeWrapper];
            VELOREN_USERDATA_STRATEGY = "system";
            postInstall = ''
              ${oldAttrs.postInstall or ""}
              if [ -f $out/bin/veloren-server-cli ]; then
                wrapProgram $out/bin/veloren-server-cli \
                  --set VELOREN_ASSETS ${veloren-assets}
              fi
            '';
          };
        };
      };
    };
}
