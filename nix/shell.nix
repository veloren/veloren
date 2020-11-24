{ nixpkgs ? <nixpkgs>, sources ? import ./sources.nix { }
, system ? builtins.currentSystem }:
let
  pkgs = import ./nixpkgs.nix { inherit sources nixpkgs system; };
  common = import ./common.nix { inherit pkgs; };
  crate2nix = pkgs.callPackage sources.crate2nix { inherit pkgs; };
in with pkgs;
mkShell {
  name = "veloren-shell";
  nativeBuildInputs = [ git git-lfs niv nixfmt crate2nix cargo rustc ];
  buildInputs = lib.concatLists (lib.attrValues common.crateDeps);
  shellHook = ''
    export LD_LIBRARY_PATH=${common.neededLibPaths}
  '';
}
