{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs.nci.url = "github:yusdacra/nix-cargo-integration";

  outputs = inputs: let
    lib = inputs.nci.inputs.nixpkgs.lib;

    git = let
      sourceInfo = inputs.self.sourceInfo;
      dateTimeFormat = import ./nix/dateTimeFormat.nix;
      dateTime = dateTimeFormat sourceInfo.lastModified;
      shortRev = sourceInfo.shortRev or "dirty";
    in {
      prettyRev = shortRev + "/" + dateTime;
      tag = "";
    };

    filteredSource = let
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
        ".cargo"
      ];
      ignorePaths = path: type: let
        split = lib.splitString "/" path;
        actual = lib.drop 4 split;
        _path = lib.concatStringsSep "/" actual;
      in
        lib.all (n: ! (lib.hasPrefix n _path)) pathsToIgnore;
    in
      builtins.path {
        name = "veloren-source";
        path = toString ./.;
        # filter out unnecessary paths
        filter = ignorePaths;
      };

    wrapWithAssets = common: _: old: let
      pkgs = common.pkgs;
      runtimeLibs = with pkgs; [
        xorg.libX11
        xorg.libXi
        xorg.libxcb
        xorg.libXcursor
        xorg.libXrandr
        libxkbcommon
        shaderc.lib
        udev
        alsa-lib
        vulkan-loader
      ];
      assets = pkgs.runCommand "veloren-assets" {} ''
        mkdir $out
        ln -sf ${./assets} $out/assets
        # check if LFS was setup properly
        checkFile="$out/assets/voxygen/background/bg_main.jpg"
        result="$(${pkgs.file}/bin/file --mime-type $checkFile)"
        if [ "$result" = "$checkFile: image/jpeg" ]; then
          echo "Git LFS seems to be setup properly."
        else
          echo "
            Git Large File Storage (git-lfs) has not been set up correctly.
            Most common reasons:
              - git-lfs was not installed before cloning this repository.
              - This repository was not cloned from the primary GitLab mirror.
              - The GitHub mirror does not support LFS.
            See the book at https://book.veloren.net/ for details.
            Run 'nix-shell -p git git-lfs --run \"git lfs install --local && git lfs fetch && git lfs checkout\"'
            or 'nix shell nixpkgs#git-lfs nixpkgs#git -c sh -c \"git lfs install --local && git lfs fetch && git lfs checkout\"'.
          "
          false
        fi
      '';
      wrapped =
        common.internal.nci-pkgs.utils.wrapDerivation old
        {nativeBuildInputs = [pkgs.makeWrapper];}
        ''
          rm -rf $out/bin
          mkdir $out/bin
          ln -sf ${old}/bin/* $out/bin/
          wrapProgram $out/bin/* \
            ${lib.optionalString (old.pname == "veloren-voxygen") "--prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}"} \
            --set VELOREN_ASSETS ${assets} \
            --set VELOREN_GIT_VERSION "${git.prettyRev}" \
            --set VELOREN_GIT_TAG "${git.tag}"
        '';
    in
      wrapped;
  in
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
          wrapper = wrapWithAssets;
        };
        veloren-server-cli.wrapper = wrapWithAssets;
      };
      overrides = {
        cCompiler = common: common.pkgs.clang;
        crates = common: prev: let
          pkgs = common.pkgs;

          configMoldLinker = ''
            touch $CARGO_HOME/config.toml
            cat >>$CARGO_HOME/config.toml <<EOF
              [target.x86_64-unknown-linux-gnu]
              linker = "clang"
              rustflags = ["-C", "link-arg=-fuse-ld=mold"]
            EOF
          '';
        in {
          veloren-common = oldAttrs: {
            # Disable `git-lfs` check here since we check it ourselves
            # We have to include the command output here, otherwise Nix won't run it
            DISABLE_GIT_LFS_CHECK = true;
            # We don't add in any information here because otherwise anything
            # that depends on common will be recompiled. We will set these in
            # our wrapper instead.
            NIX_GIT_HASH = "";
            NIX_GIT_TAG = "";
          };
          veloren-voxygen-deps = oldAttrs: {
            doCheck = false;

            nativeBuildInputs =
              (oldAttrs.nativeBuildInputs or [])
              ++ [pkgs.mold];

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
              ++ (with pkgs; [python3 pkg-config mold]);

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

            doCheck = false;

            postConfigure = ''
              ${oldAttrs.postConfigure or ""}
              ${configMoldLinker}
            '';
          };
        };
      };
    };
}
