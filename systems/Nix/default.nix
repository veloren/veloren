let
  fallbackPkgs = import <nixpkgs> {};
in

{
  alsaLib ? fallbackPkgs.alsaLib,
  atk ? fallbackPkgs.atk,
  cairo ? fallbackPkgs.cairo,
  fetchFromGitHub ? fallbackPkgs.fetchFromGitHub,
  git ? fallbackPkgs.git,
  git-lfs ? fallbackPkgs.git-lfs,
  glib ? fallbackPkgs.glib,
  gnuplot ? fallbackPkgs.gnuplot,
  gtk3 ? fallbackPkgs.gtk3,
  makeRustPlatform ? fallbackPkgs.makeRustPlatform,
  nix-gitignore ? fallbackPkgs.nix-gitignore,
  pango ? fallbackPkgs.pango,
  pkg-config ? fallbackPkgs.pkg-config,
  pkgs ? fallbackPkgs,
  rustup ? fallbackPkgs.rustup,
  stdenv ? fallbackPkgs.stdenv,
  veloren-src ? null,
}:

let
  mozRepo = fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    rev = "ac8e9d7bbda8fb5e45cae20c5b7e44c52da3ac0c";
    sha256 = "1irlkqc0jdkxdfznq7r52ycnf0kcvvrz416qc7346xhmilrx2gy6";
  };
  # `mozPkgs` is the package set of `mozRepo`; this differs from their README
  # where they use it as an overlay rather than a separate package set
  mozPkgs = import "${mozRepo}/package-set.nix" { inherit pkgs; };
  channel = mozPkgs.rustChannelOf { date = "2019-07-03"; channel = "nightly"; };
  nightlyRustPlatform = makeRustPlatform {
    rustc = channel.rust;
    cargo = channel.cargo;
  };
in

nightlyRustPlatform.buildRustPackage rec {
  name = "veloren";
  version = "unstable";
  # For information on how to automatically fetch the source from GitLab, please
  # ask @haslersn
  src = if veloren-src == null then (nix-gitignore.gitignoreSource [] ./.) else veloren-src;
  nativeBuildInputs = [
    pkg-config
    # convenience for nix-shell:
    git
    git-lfs
    gnuplot
    rustup # Needed for RLS integration in some IDEs such as vscode
  ];
  buildInputs = [
    alsaLib
    atk
    cairo
    glib
    gtk3
    pango
  ];
  preConfigure = ''
    export HOME=`mktemp -d`
  '';
  postInstall = ''
    cp -R $src/assets $out/bin/assets
  '';
  CARGO_INCREMENTAL = 1;
  cargoSha256 = "1zhsn69171wazigxxqggwqb5j8qllr5245y2w92dpnrgmdbjqyga";

  meta = {
    platforms = stdenv.lib.platforms.linux;
  };
}
