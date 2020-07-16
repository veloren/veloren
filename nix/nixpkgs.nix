{ nixpkgs ? <nixpkgs>, system ? builtins.currentSystem
, sources ? import ./sources.nix { inherit system; } }:

let
  mozPkgs = import "${sources.nixpkgsMoz}/package-set.nix" {
    pkgs = import nixpkgs { inherit system; };
  };
  rustChannel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    sha256 = "sha256-18R7sZfLGmtYkz24jUaq268fJO2A71p+dWvGm4DgqEw=";
  };
in import nixpkgs {
  inherit system;
  overlays = [
    (self: super: {
      rustc = rustChannel.rust;
      inherit (rustChannel) cargo rust rust-std rust-src;
    })
  ];
}
