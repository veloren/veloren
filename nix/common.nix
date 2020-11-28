{ system, nixpkgsMoz, nixpkgs }:
let
  mozPkgs = import "${nixpkgsMoz}/package-set.nix" {
    pkgs = import nixpkgs { inherit system; };
  };
  rustChannel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    sha256 = "sha256-P4FTKRe0nM1FRDV0Q+QY2WcC8M9IR7aPMMLWDfv+rEk=";
  };
  pkgs = import nixpkgs {
    inherit system;
    overlays = [
      (self: super: {
        rustc = rustChannel.rust;
        inherit (rustChannel)
        ;
      })
    ];
  };
in with pkgs;
let
  xorgLibraries = with xorg; [ libX11 libXcursor libXrandr libXi ];
  otherLibraries = [
    libGL
    # wayland
    /* uncomment above for wayland support (?)
       for some reason it doesn't work (triggers `unreachable!()` code in winit!)
       so I disabled it by default
    */
  ];
  neededLibPathsVoxygen = lib.concatStringsSep ":"
    (map (p: "${p}/lib") (xorgLibraries ++ otherLibraries));

  crateDeps = {
    libudev-sys = [ pkg-config libudev ];
    alsa-sys = [ pkg-config alsaLib ];
    veloren-network = [ pkg-config openssl ];
    veloren-voxygen = [ atk cairo glib gtk3 pango ];
  };
in { inherit pkgs neededLibPathsVoxygen crateDeps; }
