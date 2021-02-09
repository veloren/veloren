{ nixpkgsMoz, pkgs }:
let
  mozPkgs = import "${nixpkgsMoz}/package-set.nix" {
    inherit pkgs;
  };

  channel = mozPkgs.rustChannelOf {
    rustToolchain = ../rust-toolchain;
    sha256 = "sha256-9wp6afVeZqCOEgXxYQiryYeF07kW5IHh3fQaOKF2oRI=";
  };
in
channel // {
  rust = channel.rust.override { extensions = [ "rust-src" ]; };
}
