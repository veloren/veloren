{ nixpkgs ? <nixpkgs>, sources ? import ./sources.nix { }
, system ? builtins.currentSystem }:
let
  common = import ./common.nix {
    inherit nixpkgs system;
    inherit (sources) nixpkgsMoz;
  };

  crate2nix =
    common.pkgs.callPackage sources.crate2nix { inherit (common) pkgs; };
in with common.pkgs;
mkShell {
  name = "veloren-shell";
  nativeBuildInputs =
    [ git git-lfs niv nixfmt crate2nix cargo rustc rustfmt clippy cachix ];
  buildInputs = lib.concatLists (lib.attrValues common.crateDeps);
  shellHook = ''
    export NIX_CONFIG="
      substituters = https://cache.nixos.org https://veloren-nix.cachix.org
      trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= veloren-nix.cachix.org-1:zokfKJqVsNV6kI/oJdLF6TYBdNPYGSb+diMVQPn/5Rc=
    "
    export LD_LIBRARY_PATH=${common.neededLibPathsVoxygen}
  '';
}
