{
  description = "A flake for building the akasha Rust program.";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.follows = "rust-overlay/flake-utils";
    nixpkgs.follows = "rust-overlay/nixpkgs";
  };

  outputs = inputs: with inputs;
    {
      defaultPackage = pkgs.lib.platforms.${builtins.currentSystem}.rustPackages.${packageName} {
        inherit nixpkgs rust-overlay;
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [ pkgs.alsaLib ];
      };
    };
}