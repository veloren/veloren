{ sources ? import ./sources.nix { }, pkgs ? import <nixpkgs> { } }:

let crate2nix = import sources.crate2nix { };
in pkgs.mkShell {
  name = "veloren-shell";
  nativeBuildInputs = with pkgs; [
    pkg-config
    python3
    git
    git-lfs
    niv
    crate2nix
    rustup
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
