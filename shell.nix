with import <nixpkgs> {};

import ./default.nix {
  git = git;
  git-lfs = git-lfs;
  gnuplot = gnuplot;
  rustup = rustup;
  # The source is copied to the nix store. We don't want to do this (including assets) for every
  # time the `nix-shell` is entered. Therefore we create a source which contains only the files
  # necessary to evaluate `buildRustPackage` successfully:
  veloren-src = runCommand "veloren-shell" {} (lib.concatMapStrings
    (p: "mkdir -p $(dirname $out/${p}); cp ${./. + "/${p}"} $out/${p}\n")
    [
      "Cargo.lock"
      "Cargo.toml"
      "chat-cli/Cargo.toml"
      "chat-cli/src/main.rs"
      "client/Cargo.toml"
      "client/src/lib.rs"
      "common/Cargo.toml"
      "common/src/lib.rs"
      "server-cli/Cargo.toml"
      "server-cli/src/main.rs"
      "server/Cargo.toml"
      "server/src/lib.rs"
      "voxygen/Cargo.toml"
      "voxygen/src/main.rs"
      "world/Cargo.toml"
      "world/src/lib.rs"
      "network/Cargo.toml"
      "network/src/lib.rs"
    ]
  );
}
