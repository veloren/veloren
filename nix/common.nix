{ system, sources, nixpkgs }:
let
  mozPkgs = import "${sources.nixpkgsMoz}/package-set.nix" {
    pkgs = import nixpkgs { inherit system; };
  };

  rustChannel =
    let
      channel = mozPkgs.rustChannelOf {
        rustToolchain = ../rust-toolchain;
        sha256 = "sha256-P4FTKRe0nM1FRDV0Q+QY2WcC8M9IR7aPMMLWDfv+rEk=";
      };
      flip = f: a: b: f b a;
      mapAttrs = builtins.mapAttrs;
    in
    flip mapAttrs channel (name: value:
      (if name == "rust" then
        value.override { extensions = [ "rust-src" ]; }
      else
        value));

  pkgs = import nixpkgs {
    inherit system;
    overlays = [
      (final: prev: {
        rustc = rustChannel.rust;
        inherit (rustChannel)
          ;
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
{ inherit pkgs voxygenNeededLibs crateDeps gitLfsCheckFile; }
