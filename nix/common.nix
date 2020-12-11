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
      })
    ];
  };
in
with pkgs;
{
  inherit pkgs;
  # deps that crates need (for compiling)
  crateDeps =
    let
      makeDeps = b: n: { buildInputs = b; nativeBuildInputs = n; };
    in
    {
      libudev-sys = makeDeps [ libudev ] [ pkg-config ];
      alsa-sys = makeDeps [ alsaLib ] [ pkg-config ];
      veloren-network = makeDeps [ openssl ] [ pkg-config ];
      veloren-voxygen = makeDeps [ xorg.libxcb ] [ ];
    };
  # deps that voxygen needs to function
  # FIXME: Wayland doesn't work (adding libxkbcommon, wayland and wayland-protocols results in a panic)
  voxygenNeededLibs = (with xorg; [ libX11 libXcursor libXrandr libXi ])
    ++ [ libGL ];
  gitLfsCheckFile = ../assets/voxygen/background/bg_main.png;
}
