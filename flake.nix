{
  description = "Headless audio recording application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-21.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };
        rustc = pkgs.rust-bin.stable.latest.default;
        cargo = pkgs.buildPackages.cargo;
      in
      rec {
        packages = {
          akasha = pkgs.stdenv.mkDerivation {
            name = "akasha";
            src = ./.;
            buildInputs = [ cargo rustc pkgs.alsaLib pkgs.pkg-config ];

            buildPhase = ''
              cargo build --release
            '';

            installPhase = ''
              install -D -m755 target/release/akasha $out/bin/akasha
            '';

            meta = with pkgs.lib; {
              homepage = "https://github.com/alxpettit/akasha";
              description = "Headless audio recording application";
              license = licenses.agpl3Plus;
              platforms = platforms.all;
            };
          };
        };

        defaultPackage = packages.akasha;
      });
}
