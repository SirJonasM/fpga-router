{
  description = "A reproducible Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # 1. Add the Rust Overlay for better toolchain management
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # 2. Apply the overlay to pkgs
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # 3. Define our toolchains
        # 'complete' includes rust-analyzer/rustfmt for local dev
        devToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };
        
        # 'minimal' is just what we need for CI (faster download)
        ciToolchain = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [ "clippy" ];
        };

        # Common libraries needed for both
        commonInputs = with pkgs; [
          # Add things like openssl, pkg-config, or zlib here if needed
        ];
      in
      {
        devShells = {
          # Local development: 'nix develop'
          default = pkgs.mkShell {
            nativeBuildInputs = [ devToolchain pkgs.python3 pkgs.typst ];
            buildInputs = commonInputs;
            shellHook = ''echo "Welcome to your FULL dev shell (with rust-analyzer)!"'';
          };

          # CI Environment: 'nix develop .#ci'
          ci = pkgs.mkShell {
            nativeBuildInputs = [ ciToolchain ];
            buildInputs = commonInputs;
            shellHook = ''echo "CI environment loaded."'';
          };
        };
      });
}
