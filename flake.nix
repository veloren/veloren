{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    naersk = {
      url = "github:yusdacra/naersk/veloren";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flakeUtils.url = "github:numtide/flake-utils";
    rustOverlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = inputs: with inputs; with flakeUtils.lib;
    eachSystem [ "x86_64-linux" ] (system:
      let
        common = import ./nix/common.nix {
          sources = {
            inherit
              naersk
              nixpkgs
              rustOverlay
              ;
          };
          inherit system;
        };

        mkPackage = crateName: import ./nix/build.nix {
          inherit common;
          crateToBuild = crateName;
          sourceInfo =
            if self.sourceInfo ? rev
            then self.sourceInfo // {
              # Tag would have to be set manually for stable releases flake
              # because there's currently no way to get the tag via the interface.
              # tag = v0.8.0;
            }
            else (throw "Can't get revision because the git tree is dirty");
        };

        cratesToPackage = [ "veloren-voxygen" "veloren-server-cli" ];

        genAttrs = names: f:
          builtins.listToAttrs (map (n: { name = n; value = f n; }) names);
      in
      rec {
        packages = genAttrs cratesToPackage mkPackage;
        defaultPackage = packages.veloren-voxygen;

        apps = builtins.mapAttrs (n: v: mkApp { name = n; drv = v; }) packages;
        defaultApp = apps.veloren-voxygen;

        devShell = import ./nix/devShell.nix {
          inherit common;
        };
      }
    );
}
