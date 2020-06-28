{ sources ? import ./sources.nix { }, nixpkgsSrc ? <nixpkgs> }:

let
  mozPkgs = import "${sources.nixpkgsMoz}/package-set.nix" {
    pkgs = import nixpkgsSrc { };
  };
  rustChannel = mozPkgs.rustChannelOf { rustToolchain = ../rust-toolchain; };
in import nixpkgsSrc {
  overlays = [
    (self: super: {
      rustc = rustChannel.rust;
      inherit (rustChannel) cargo rust rust-std rust-src;
    })
  ];
}
