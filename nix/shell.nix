{ nixpkgs ? <nixpkgs>, sources ? import ./sources.nix { }
, system ? builtins.currentSystem }:

let
  pkgs = import ./nixpkgs.nix { inherit sources nixpkgs system; };
  crate2nix = import sources.crate2nix { inherit pkgs; };
in pkgs.mkShell {
  name = "veloren-shell";
  nativeBuildInputs = with pkgs; [
    pkg-config
    python3
    git
    git-lfs
    niv
    nixfmt
    crate2nix
    cargo
    rustc
  ];
  buildInputs = with pkgs; [
    alsaLib
    atk
    cairo
    glib
    gtk3
    libudev
    openssl
    pango
  ];
}
