{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs.nci.url = "github:yusdacra/nix-cargo-integration";

  outputs = inputs: let
    lib = inputs.nci.inputs.nixpkgs.lib;
    ncl = inputs.nci.lib.nci-lib;

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
        ".github"
        ".gitlab"
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
    checkIfLfsIsSetup = pkgs: checkFile: ''
      checkFile="${checkFile}"
      result="$(${pkgs.file}/bin/file --mime-type $checkFile)"
      if [ "$result" = "$checkFile: image/jpeg" ]; then
        echo "Git LFS seems to be setup properly."
        true
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
  in
    inputs.nci.lib.makeOutputs {
      root = ./.;
      config = common: {
        cCompiler.package = common.pkgs.clang;
        outputs.defaults = {
          package = "veloren-voxygen";
          app = "veloren-voxygen";
        };
        shell = {
          startup.checkLfsSetup.text = ''
            ${checkIfLfsIsSetup common.pkgs "$PWD/assets/voxygen/background/bg_main.jpg"}
            if [ $? -ne 0 ]; then
              exit 1
            fi
          '';
        };
      };
      pkgConfig = common: let
        inherit (common) pkgs;
        veloren-common-ov = {
          # Disable `git-lfs` check here since we check it ourselves
          # We have to include the command output here, otherwise Nix won't run it
          DISABLE_GIT_LFS_CHECK = true;
          # We don't add in any information here because otherwise anything
          # that depends on common will be recompiled. We will set these in
          # our wrapper instead.
          NIX_GIT_HASH = "";
          NIX_GIT_TAG = "";
        };
        assets = pkgs.runCommand "veloren-assets" {} ''
          mkdir $out
          ln -sf ${./assets} $out/assets
          ${checkIfLfsIsSetup pkgs "$out/assets/voxygen/background/bg_main.jpg"}
        '';
        wrapWithAssets = _: old: let
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
          wrapped =
            common.internal.pkgsSet.utils.wrapDerivation old
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
      in {
        veloren-voxygen = let
          veloren-voxygen-deps-ov = oldAttrs: {
            buildInputs = ncl.addBuildInputs oldAttrs (
              with pkgs; [
                alsa-lib
                libxkbcommon
                udev
                xorg.libxcb
              ]
            );
            nativeBuildInputs =
              ncl.addNativeBuildInputs oldAttrs (with pkgs; [python3 pkg-config]);

            SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";
            VELOREN_ASSETS = "${assets}";

            doCheck = false;
            dontCheck = true;
          };
        in {
          features = {
            release = ["default-publish"];
            dev = ["default-publish"];
            test = ["default-publish"];
          };
          depsOverrides.fix-build.overrideAttrs = veloren-voxygen-deps-ov;
          overrides = {
            fix-veloren-common = veloren-common-ov;
            add-deps-reqs.overrideAttrs = veloren-voxygen-deps-ov;
            fix-build.overrideAttrs = prev: {
              src = filteredSource;

              VELOREN_USERDATA_STRATEGY = "system";

              dontUseCmakeConfigure = true;

              preConfigure = ''
                ${prev.preConfigure or ""}
                substituteInPlace voxygen/src/audio/soundcache.rs \
                  --replace \
                  "../../../assets/voxygen/audio/null.ogg" \
                  "${./assets/voxygen/audio/null.ogg}"
              '';
            };
          };
          wrapper = wrapWithAssets;
        };
        veloren-server-cli = let
          veloren-server-cli-deps-ov = oldAttrs: {
            doCheck = false;
            dontCheck = true;
          };
        in {
          features = {
            release = ["default-publish"];
            dev = ["default-publish"];
            test = ["default-publish"];
          };
          depsOverrides.fix-build.overrideAttrs = veloren-server-cli-deps-ov;
          overrides = {
            fix-veloren-common = veloren-common-ov;
            add-deps-reqs.overrideAttrs = veloren-server-cli-deps-ov;
            fix-build = {
              src = filteredSource;
              VELOREN_USERDATA_STRATEGY = "system";
            };
          };
          wrapper = wrapWithAssets;
        };
      };
    };
}
