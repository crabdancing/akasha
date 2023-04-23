{
  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/unstable";
    flake-utils.follows = "cargo2nix/flake-utils";
    nixpkgs.follows = "cargo2nix/nixpkgs";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [cargo2nix.overlays.default];
        };

        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustVersion = "1.65.0";
          packageFun = import ./Cargo.nix;

          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
            (pkgs.rustBuilder.rustLib.makeOverride {
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.alsaLib pkgs.pkg-config pkgs.libogg
                ];
              };
            })
          ];

        };



      in rec {
        packages = {
          akasha = (rustPkgs.workspace.akasha {});
          default = packages.akasha;
        };
      }
    );
}

# {
#   description = "Headless audio recording application";

#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixos-21.11";
#     flake-utils.url = "github:numtide/flake-utils";
#     rust-overlay.url = "github:oxalica/rust-overlay";
#   };

#   outputs = { self, nixpkgs, flake-utils, rust-overlay }:
#     flake-utils.lib.eachDefaultSystem (system:
#       let
#         pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };
#         rustc = pkgs.rust-bin.stable.latest.default;
#         cargo = pkgs.buildPackages.cargo;
#       in
#       rec {
#         packages = {
#           akasha = pkgs.stdenv.mkDerivation {
#             name = "akasha";
#             src = ./.;
#             buildInputs = [ cargo rustc pkgs.alsaLib pkgs.pkg-config ];

#             buildPhase = ''
#               export CARGO_HOME=$(mktemp -d)
#               cargo build --release
#             '';

#             installPhase = ''
#               install -D -m755 target/release/akasha $out/bin/akasha
#             '';

#             meta = with pkgs.lib; {
#               homepage = "https://github.com/alxpettit/akasha";
#               description = "Headless audio recording application";
#               license = licenses.agpl3Plus;
#               platforms = platforms.all;
#             };
#           };
#         };

#         defaultPackage = packages.akasha;
#       });
# }
