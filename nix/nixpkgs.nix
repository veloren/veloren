{ system, sources ? import ./sources.nix { inherit system; }
, nixpkgs ? sources.nixpkgs }:

let
  mozPkgs = import "${sources.nixpkgsMoz}/package-set.nix" {
    pkgs = import nixpkgs { inherit system; };
  };
  rustChannel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    hash = "sha256-P4FTKRe0nM1FRDV0Q+QY2WcC8M9IR7aPMMLWDfv+rEk=";
  };
in import nixpkgs {
  inherit system;
  overlays = [
    (self: super: {
      rustc = rustChannel.rust;
      inherit (rustChannel)
      ;
    })
  ];
}
