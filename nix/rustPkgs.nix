{ nixpkgsMoz, pkgs }:
let
  mozPkgs = import "${nixpkgsMoz}/package-set.nix" {
    inherit pkgs;
  };

  channel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    sha256 = "sha256-P4FTKRe0nM1FRDV0Q+QY2WcC8M9IR7aPMMLWDfv+rEk=";
  };

in
channel // {
  rust = channel.rust.override { extensions = [ "rust-src" ]; };
}
