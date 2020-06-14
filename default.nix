let
  fallbackPkgs = import <nixpkgs> {};
  fallbackMozSrc = builtins.fetchTarball "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz";
in

{
  alsaLib ? fallbackPkgs.alsaLib,
  atk ? fallbackPkgs.atk,
  cairo ? fallbackPkgs.cairo,
  git ? null,
  git-lfs ? null,
  glib ? fallbackPkgs.glib,
  gnuplot ? null,
  gtk3 ? fallbackPkgs.gtk3,
  libudev ? fallbackPkgs.libudev,
  makeRustPlatform ? fallbackPkgs.makeRustPlatform,
  mozSrc ? fallbackMozSrc,
  nix-gitignore ? fallbackPkgs.nix-gitignore,
  openssl ? fallbackPkgs.openssl,
  pango ? fallbackPkgs.pango,
  pkg-config ? fallbackPkgs.pkg-config,
  pkgs ? fallbackPkgs,
  python3 ? fallbackPkgs.python3,
  rustup ? null,
  stdenv ? fallbackPkgs.stdenv,
  veloren-src ? null,
}:

let
  # `mozPkgs` is the package set of `mozRepo`; this differs from their README
  # where they use it as an overlay rather than a separate package set
  mozPkgs = import "${mozSrc}/package-set.nix" { inherit pkgs; };
  channel = mozPkgs.rustChannelOf { rustToolchain = ./rust-toolchain; };
  rustPlatform = makeRustPlatform {
    rustc = channel.rust;
    cargo = channel.cargo;
  };
in

rustPlatform.buildRustPackage rec {
  pname = "veloren";
  version = "unstable";
  # For information on how to automatically fetch the source from GitLab, please
  # ask @haslersn
  src = if veloren-src == null then (nix-gitignore.gitignoreSource [] ./.) else veloren-src;
  nativeBuildInputs = [
    pkg-config
    python3
    # Convenience for nix-shell
    git
    git-lfs
    gnuplot
    rustup # Required for integration in some editors
  ];
  buildInputs = [
    alsaLib
    atk
    cairo
    glib
    gtk3
    pango
    libudev
    openssl
  ];
  #preConfigure = "export HOME=`mktemp -d`";
  postInstall = "cp -R $src/assets $out/bin/assets";
  # If veloren-vendor build fails with hash mismatch, change this hash with `got:` hash
  cargoSha256 = "13aa2jypqhg4y7bpkxqdchd0sw85hq6galafswbg1d4bjwphnq70";

  meta = {
    platforms = stdenv.lib.platforms.linux;
  };
}
