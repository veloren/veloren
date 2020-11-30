{ nixpkgs ? <nixpkgs>
, sources ? import ./sources.nix { }
, system ? builtins.currentSystem
, nvidia ? false
}:
let common = import ./common.nix { inherit nixpkgs system sources; };
in
with common.pkgs;
let
  nixGLPackages = ((with nixGL; [ nixGLIntel ]) ++ (lib.optional nvidia
    (with nixGL; [ nixGLNvidia nixGLNvidiaBumblebee ])));

  getAllCratesDeps = name:
    (lib.concatLists
      (map (attrset: attrset."${name}") (lib.attrValues common.crateDeps)));

  bundleCrate = writeScriptBin "bundleCrate" ''
    #!${stdenv.shell}
    ${nix-bundle}/bin/nix-bundle "(pkgs.callPackage ./nix/default.nix { }).$1" /bin/$1
  '';
in
with common;
mkShell {
  name = "veloren-shell";
  nativeBuildInputs = [
    bundleCrate
    git
    git-lfs
    niv
    nixpkgs-fmt
    crate2nix
    cargo
    rustc
    rustfmt
    clippy
    cachix
  ] ++ nixGLPackages ++ (getAllCratesDeps "nativeBuildInputs");
  buildInputs = getAllCratesDeps "buildInputs";
  shellHook = ''
    # Setup our cachix "substituter"
    export NIX_CONFIG="
      substituters = https://cache.nixos.org https://veloren-nix.cachix.org
      trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= veloren-nix.cachix.org-1:zokfKJqVsNV6kI/oJdLF6TYBdNPYGSb+diMVQPn/5Rc=
    "
    # We need this so that Voxygen runs
    export LD_LIBRARY_PATH=${lib.makeLibraryPath voxygenNeededLibs}

    # No need to install git-lfs and run fetch / checkout commands if we have it setup
    [ "$(${file}/bin/file --mime-type ${gitLfsCheckFile})" = "${gitLfsCheckFile}: image/png" ] || (git lfs install --local && git lfs fetch && git lfs checkout)
  '';
}
