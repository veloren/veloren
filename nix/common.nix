{ pkgs }:
with pkgs;
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
  neededLibPaths = lib.concatStringsSep ":"
    (map (p: "${p}/lib") (xorgLibraries ++ otherLibraries));

  crateDeps = {
    libudev-sys = [ pkg-config libudev ];
    alsa-sys = [ pkg-config alsaLib ];
    veloren-network = [ pkg-config openssl ];
    veloren-voxygen = [ atk cairo glib gtk3 pango ];
  };
in { inherit neededLibPaths crateDeps; }
