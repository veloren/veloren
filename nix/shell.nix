{ nixpkgs ? <nixpkgs>, sources ? import ./sources.nix { }
, system ? builtins.currentSystem }:
let
  common = import ./common.nix { inherit nixpkgs sources system; };
  inherit (common) pkgs;
  crate2nix = pkgs.callPackage sources.crate2nix { inherit pkgs; };
in with pkgs;
mkShell {
  name = "veloren-shell";
  nativeBuildInputs =
    [ git git-lfs niv nixfmt crate2nix cargo rustc rustfmt clippy ];
  buildInputs = lib.concatLists (lib.attrValues common.crateDeps);
  shellHook = ''
    export LD_LIBRARY_PATH=${common.neededLibPaths}
  '';
}
