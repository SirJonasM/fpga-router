{
  description = "A reproducible Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          # nativeBuildInputs are for tools needed at build time (like compilers)
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            rustfmt
          ];

          # buildInputs are for libraries your code links against
          buildInputs = with pkgs; [
            # Add other system libraries here
          ];

          shellHook = ''
            echo "Welcome to your Flake-powered Rust dev shell!"
          '';
        };
      });
}
