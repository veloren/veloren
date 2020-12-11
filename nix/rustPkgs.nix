{ nixpkgsMoz, pkgs }:
let
  mozPkgs = import "${nixpkgsMoz}/package-set.nix" {
    inherit pkgs;
  };

  channel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    sha256 = "sha256-kDtMqYvrTbahqYHYFQOWyvT0+F5o4UVcqkMZt0c43kc=";
  };

in
channel // {
  rust = channel.rust.override { extensions = [ "rust-src" ]; };
}
