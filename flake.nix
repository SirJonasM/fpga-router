{
  description = "A reproducible Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        devToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };
        
        # --- NEW: Graphics Libraries for NixOS ---
        runtimeLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          vulkan-loader # Critical for Vello/Wgpu
        ];

        commonInputs = with pkgs; [
          cmake
          pkg-config # Essential for finding system libs
        ] ++ runtimeLibs;
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ 
            devToolchain 
            pkgs.python3 
            pkgs.typst 
          ];
          buildInputs = commonInputs;

          # --- NEW: Inject libraries into the runtime path ---
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH
            echo "Welcome to your FULL dev shell! Wayland/Vulkan libs loaded."
          '';
        };
      });
}
