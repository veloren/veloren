{ common }:
with common; with pkgs;
mkShell {
  name = "veloren-shell";
  nativeBuildInputs = [
    cachix
    git
    git-lfs
    nixpkgs-fmt
    rustc
  ] ++ crateDeps.nativeBuildInputs;
  buildInputs = crateDeps.buildInputs;
  shellHook = ''
    # Setup our cachix "substituter"
    export NIX_CONFIG="
      substituters = https://cache.nixos.org https://veloren-nix.cachix.org
      trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= veloren-nix.cachix.org-1:zokfKJqVsNV6kI/oJdLF6TYBdNPYGSb+diMVQPn/5Rc=
    "
    # Add libraries Voxygen needs so that it runs
    export LD_LIBRARY_PATH=${lib.makeLibraryPath voxygenNeededLibs}

    # No need to install git-lfs and run fetch / checkout commands if we have it setup
    [ "$(${file}/bin/file --mime-type ${gitLfsCheckFile})" = "${gitLfsCheckFile}: image/png" ] || (git lfs install --local && git lfs fetch && git lfs checkout)
  '';
}
