{ sources, system }:
let
  pkgz = import sources.nixpkgs { inherit system; overlays = [ sources.rustOverlay.overlay ]; };
  rust = (pkgz.rust-bin.fromRustupToolchainFile ../rust-toolchain).override {
    extensions = [ "rust-src" ];
  };

  pkgs = import sources.nixpkgs {
    inherit system;
    overlays = [
      (final: prev: {
        rustc = rust;
      })
      (final: prev: {
        naersk = prev.callPackage sources.naersk { };
      })
    ];
  };
in
with pkgs;
{
  inherit pkgs;
  # deps that crates need (for compiling)
  crateDeps =
    {
      nativeBuildInputs = [ pkg-config python3 ];
      buildInputs = [ libudev alsaLib openssl xorg.libxcb ];
    };
  # deps that voxygen needs to function
  # FIXME: Wayland doesn't work (adding libxkbcommon, wayland and wayland-protocols results in a panic)
  voxygenNeededLibs = (with xorg; [ libX11 libXcursor libXrandr libXi ])
    ++ [ libGL ];
  gitLfsCheckFile = ../assets/voxygen/background/bg_main.png;
}
