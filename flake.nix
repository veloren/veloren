{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs.nci.url = "github:yusdacra/nix-cargo-integration";

  outputs = inputs: let
    lib = inputs.nci.inputs.nixpkgs.lib;
    outputs = inputs.nci.lib.makeOutputs {
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

          prettyRev = with sourceInfo;
            builtins.substring 0 8 rev
            + "/"
            + utils.dateTimeFormat lastModified;

          tag =
            if sourceInfo ? tag
            then sourceInfo.tag
            else "";

          configMoldLinker = ''
            cat >>$CARGO_HOME/config.toml <<EOF
              [target.x86_64-unknown-linux-gnu]
              linker = "clang"
              rustflags = ["-C", "link-arg=-fuse-ld=${lib.getExe pkgs.mold}"]
            EOF
          '';

          pathsToIgnore = [
            "flake.nix"
            "flake.lock"
            "nix"
            "assets"
            "README.md"
            "CONTRIBUTING.md"
            "CHANGELOG.md"
            "CODE_OF_CONDUCT.md"
            "clippy.toml"
          ];
          ignorePaths = path: type: let
            split = lib.splitString "/" path;
            actual = lib.drop 4 split;
            _path = lib.concatStringsSep "/" actual;
          in
            lib.all (n: ! (lib.hasPrefix n _path)) pathsToIgnore;
          filteredSource = builtins.path {
            name = "veloren-source";
            path = toString ./.;
            # filter out unnecessary paths
            filter = ignorePaths;
          };
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
            doCheck = false;

            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
          };
          veloren-voxygen = oldAttrs: {
            src = filteredSource;

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
              wrapProgram $out/bin/veloren-voxygen \
                --set LD_LIBRARY_PATH ${lib.makeLibraryPath common.runtimeLibs}
            '';
          };
          veloren-server-cli-deps = oldAttrs: {
            doCheck = false;

            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
          };
          veloren-server-cli = oldAttrs: {
            src = filteredSource;

            VELOREN_USERDATA_STRATEGY = "system";

            nativeBuildInputs =
              (oldAttrs.nativeBuildInputs or [])
              ++ [pkgs.makeWrapper];

            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
            postInstall = ''
              ${oldAttrs.postInstall or ""}
            '';
          };
        };
      };
    };
    wrapWithAssets = system: old: let
      pkgs = inputs.nci.inputs.nixpkgs.legacyPackages.${system};
      assets = pkgs.runCommand "veloren-assets" {} ''
        mkdir $out
        ln -sf ${./assets} $out/assets
      '';
      wrapped =
        pkgs.runCommand "${old.name}-wrapped"
        {
          inherit (old) pname version meta;
          nativeBuildInputs = [pkgs.makeWrapper];
        }
        ''
          mkdir -p $out
          ln -sf ${old}/* $out/
          rm -rf $out/bin
          mkdir $out/bin
          ln -sf ${old}/bin/* $out/bin/
          wrapProgram $out/bin/* --set VELOREN_ASSETS ${assets}
        '';
    in
      wrapped;
  in
    outputs
    // rec {
      apps =
        lib.mapAttrs
        (system: _: rec {
          default = veloren;
          veloren = {
            type = "app";
            program = lib.getExe packages.${system}.veloren-voxygen;
          };
          veloren-server = {
            type = "app";
            program = lib.getExe packages.${system}.veloren-server-cli;
          };
        })
        outputs.apps;
      packages =
        lib.mapAttrs
        (system: pkgs: rec {
          default = veloren-voxygen;
          veloren-voxygen = wrapWithAssets system pkgs.veloren-voxygen;
          veloren-voxygen-debug = wrapWithAssets system pkgs.veloren-voxygen-debug;
          veloren-server-cli = wrapWithAssets system pkgs.veloren-server-cli;
          veloren-server-cli-debug = wrapWithAssets system pkgs.veloren-server-cli-debug;
        })
        outputs.packages;
    };
}
