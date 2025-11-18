{
  description = "demo-iced";

  nixConfig = {
    extra-substituters = [
      "https://mirrors.ustc.edu.cn/nix-channels/store"
    ];
    trusted-substituters = [
      "https://mirrors.ustc.edu.cn/nix-channels/store"
    ];
  };


  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    nur.url = "github:nix-community/NUR";
  };

  outputs = { self, nur, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) (nur.overlays) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.nightly.latest.default.override {
            extensions = [ "rust-src" ];
          }
        );
        puffin_viewer = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "puffin_viewer";
          version = "0.22.0";

          cargoBuildFlags = [ "-p puffin_viewer" ];
          cargoPatches = [ ./puffin-Cargo.lock.patch ];

          src = pkgs.fetchFromGitHub {
            owner = "EmbarkStudios";
            repo = "puffin";
            rev = "puffin_viewer-0.22.0";
            hash = "sha256-ppE/f6jLRe6a1lfUQUlxTq/L29DwAD/a58u5utUJMoU=";
          };

          cargoHash = "sha256-zhijQ+9vVB4IL/t1+IGLAnvJka0AB1yJRWo/qEyUfx0=";
        });
      in
      {
        devShells.default = craneLib.devShell {
          buildInputs = with pkgs; [];
          packages = [ puffin_viewer ];
        };
      }
    );
}
