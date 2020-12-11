{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    crate2nix = {
      url = "github:kolloch/crate2nix?rev=3701179c8aef0677dab1915457ca0f367f2dc523";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgsMoz = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
    nixGL = {
      url = "github:guibou/nixGL?rev=7d6bc1b21316bab6cf4a6520c2639a11c25a220e";
      flake = false;
    };
    nixpkgs.url = "github:NixOS/nixpkgs?rev=c00959877fb06b09468562518b408acda886c79e";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = inputs.nixpkgs.legacyPackages."${system}";

        sources = {
          inherit
            crate2nix
            nixGL
            nixpkgs
            nixpkgsMoz
            ;
        };

        veloren = import ./nix/veloren.nix {
          inherit
            nixpkgs
            sources
            system
            ;
          sourceInfo =
            if self.sourceInfo ? rev then self.sourceInfo // {
              # Tag would have to be set manually for stable releases flake
              # because there's currently no way to get the tag via the interface.
              # tag = v0.8.0;
            } else (throw "Can't get revision because the git tree is dirty");
        };

      in
      with flake-utils; rec {
        apps = builtins.mapAttrs
          (name: value: lib.mkApp { inherit name; drv = value; })
          packages;
        defaultApp = apps.veloren-voxygen;

        packages = veloren;
        defaultPackage = packages.veloren-voxygen;

        devShell = import ./nix/devShell.nix {
          inherit
            nixpkgs
            sources
            system
            ;
        };
      }
    );
}
