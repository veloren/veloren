{ nixpkgs, sources, system }:
let
  rustChannel = import ./rustPkgs.nix {
    pkgs = import nixpkgs { inherit system; };
    inherit (sources) nixpkgsMoz;
  };

  pkgs = import nixpkgs {
    inherit system;
    overlays = [
      (final: prev: {
        rustc = rustChannel.rust;
        crate2nix = prev.callPackage sources.crate2nix { pkgs = prev; };
        nixGL = prev.callPackage sources.nixGL { pkgs = prev; };
      })
    ];
  };

in
with pkgs;
let
  # deps that crates need (for compiling)
  crateDeps = {
    libudev-sys = {
      buildInputs = [ libudev ];
      nativeBuildInputs = [ pkg-config ];
    };
    alsa-sys = {
      buildInputs = [ alsaLib ];
      nativeBuildInputs = [ pkg-config ];
    };
    veloren-network = {
      buildInputs = [ openssl ];
      nativeBuildInputs = [ pkg-config ];
    };
    veloren-voxygen = {
      buildInputs = [ xorg.libxcb ];
      nativeBuildInputs = [ ];
    };
  };

  # deps that voxygen needs to function
  # FIXME: Wayland doesn't work (adding libxkbcommon, wayland and wayland-protocols results in a panic)
  voxygenNeededLibs = (with xorg; [ libX11 libXcursor libXrandr libXi ])
    ++ [ libGL ];

  gitLfsCheckFile = ../assets/voxygen/background/bg_main.png;
in
{
  inherit
    crateDeps
    gitLfsCheckFile
    pkgs
    voxygenNeededLibs
    ;
}
