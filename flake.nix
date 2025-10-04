{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci = {
      url = "github:90-008/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.parts.follows = "parts";
      inputs.dream2nix.follows = "d2n";
      inputs.crane.follows = "crane";
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    d2n = {
      url = "github:NeuralModder/dream2nix/git-fetcher-no-shallow";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane/v0.21.0";
      flake = false;
    };
  };

  outputs = inp: let
    lib = inp.nci.inputs.nixpkgs.lib;

    git = let
      sourceInfo = inp.self.sourceInfo;
      shortRev = lib.strings.concatStrings (lib.lists.take 8 (lib.strings.stringToCharacters (sourceInfo.rev or sourceInfo.dirtyRev)));
    in {
      version = "/" + shortRev + "/" + toString sourceInfo.lastModified;
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
              --set VELOREN_GIT_VERSION "${git.version}" \
          '';
        veloren-common-env = {
          # We don't add in any information here because otherwise anything
          # that depends on common will be recompiled. We will set these in
          # our wrapper instead.
          VELOREN_GIT_VERSION = "";
          VELOREN_USERDATA_STRATEGY = "system";
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
          VELOREN_ASSETS = "";
          shellHook = ''
            ${checkIfLfsIsSetup "$PWD/assets/voxygen/background/bg_main.jpg"}
            if [ $? -ne 0 ]; then
              exit 1
            fi
            export VELOREN_ASSETS="$PWD/assets"
            export VELOREN_GIT_VERSION="${git.version}"
          '';
        });

        nci.projects."veloren" = {
          export = false;
          path = filteredSource;
        };
        nci.crates."veloren-server-cli" = rec {
          profiles = {
            release.features = ["default-publish"];
            release.runTests = false;
            dev.features = ["default-publish"];
            dev.runTests = false;
          };
          depsDrvConfig.mkDerivation.nativeBuildInputs = [pkgs.mold];
          drvConfig = {
            mkDerivation = depsDrvConfig.mkDerivation;
            env = veloren-common-env;
          };
        };
        nci.crates."veloren-voxygen" = rec {
          profiles = {
            release.features = ["default-publish"];
            release.runTests = false;
            dev.features = ["default-publish"];
            dev.runTests = false;
          };
          runtimeLibs = with pkgs; [
            wayland
            wayland-protocols
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
          depsDrvConfig = {
            env =
              veloren-common-env
              // {
                SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";
              };
            mkDerivation = {
              buildInputs = with pkgs; [
                alsa-lib
                libxkbcommon
                udev
                xorg.libxcb

                fontconfig
              ];
              nativeBuildInputs = with pkgs; [
                python3
                pkg-config
                cmake
                gnumake
                mold
              ];
            };
          };
          drvConfig = {
            env =
              depsDrvConfig.env
              // {
                dontUseCmakeConfigure = true;
                VOXYGEN_NULL_SOUND_PATH = ./assets/voxygen/audio/null.ogg;
              };
            mkDerivation =
              depsDrvConfig.mkDerivation
              // {
                prePatch = ''
                                sed -i 's:"../../../assets/voxygen/audio/null.ogg":env!("VOXYGEN_NULL_SOUND_PATH"):' \
                  voxygen/src/audio/soundcache.rs
                '';
              };
            rust-crane.buildFlags = ["--bin=veloren-voxygen"];
          };
        };
      };
    };
}
