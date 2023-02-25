{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";

  outputs = inp: let
    lib = inp.nci.inputs.nixpkgs.lib;

    git = let
      sourceInfo = inp.self.sourceInfo;
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
  in
    inp.parts.lib.mkFlake {inputs = inp;} {
      imports = [inp.nci.flakeModule];
      systems = ["x86_64-linux"];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: let
        checkIfLfsIsSetup = checkFile: ''
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
        assets = pkgs.runCommand "veloren-assets" {} ''
          mkdir $out
          ln -sf ${./assets} $out/assets
          ${checkIfLfsIsSetup "$out/assets/voxygen/background/bg_main.jpg"}
        '';
        wrapWithAssets = old:
          pkgs.runCommand
          old.name
          {
            inherit (old) pname version;
            meta = old.meta or {};
            passthru =
              (old.passthru or {})
              // {
                unwrapped = old;
              };
            nativeBuildInputs = [pkgs.makeWrapper];
          }
          ''
            cp -rs --no-preserve=mode,ownership ${old} $out
            wrapProgram $out/bin/* \
              --set VELOREN_ASSETS ${assets} \
              --set VELOREN_GIT_VERSION "${git.prettyRev}" \
              --set VELOREN_GIT_TAG "${git.tag}"
          '';
        veloren-common-ov = {
          # We don't add in any information here because otherwise anything
          # that depends on common will be recompiled. We will set these in
          # our wrapper instead.
          NIX_GIT_HASH = "";
          NIX_GIT_TAG = "";
        };
        voxygenOut = config.nci.outputs."veloren-voxygen";
        serverCliOut = config.nci.outputs."veloren-server-cli";
      in {
        packages.veloren-voxygen = wrapWithAssets voxygenOut.packages.release;
        packages.veloren-voxygen-dev = wrapWithAssets voxygenOut.packages.dev;
        packages.veloren-server-cli = wrapWithAssets serverCliOut.packages.release;
        packages.veloren-server-cli-dev = wrapWithAssets serverCliOut.packages.dev;
        packages.default = config.packages."veloren-voxygen";

        devShells.default = config.nci.outputs."veloren".devShell.overrideAttrs (old: {
          shellHook = ''
            ${checkIfLfsIsSetup "$PWD/assets/voxygen/background/bg_main.jpg"}
            if [ $? -ne 0 ]; then
              exit 1
            fi
          '';
        });

        nci.projects."veloren".relPath = "";
        nci.crates."veloren-server-cli" = let
          veloren-server-cli-deps-ov = _: {
            doCheck = false;
            dontCheck = true;
          };
        in {
          profiles = {
            release.features = ["default-publish"];
            dev.features = ["default-publish"];
          };
          depsOverrides.fix-build.overrideAttrs = veloren-server-cli-deps-ov;
          overrides = {
            fix-veloren-common = veloren-common-ov;
            add-deps-reqs.overrideAttrs = veloren-server-cli-deps-ov;
            fix-build.override = _: {
              src = filteredSource;
              VELOREN_USERDATA_STRATEGY = "system";
            };
          };
        };
        nci.crates."veloren-voxygen" = let
          veloren-voxygen-deps-ov = prev: {
            buildInputs =
              (prev.buildInputs or [])
              ++ (
                with pkgs; [
                  alsa-lib
                  libxkbcommon
                  udev
                  xorg.libxcb
                ]
              );
            nativeBuildInputs =
              (prev.nativeBuildInputs or [])
              ++ (
                with pkgs; [
                  python3
                  pkg-config
                  cmake
                  gnumake
                ]
              );

            SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";
            VELOREN_ASSETS = "${assets}";

            checkPhase = ":";
          };
        in {
          profiles = {
            release.features = ["default-publish"];
            dev.features = ["default-publish"];
          };
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
            stdenv.cc.cc.lib
          ];
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
        };
      };
    };
}
