{
  description = "A Rust library and CLI for parsing MTK logo images";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      allSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f rec {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      });
    in
    {
      packages = forAllSystems ({ pkgs, rustPlatform, ... }: {
        default = rustPlatform.buildRustPackage {
          pname = "mtklogo";
          version = "1.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          buildAndTestSubdir = "cli";

          postInstall = ''
            cp cli/resources/bin/mtklogo.yaml $out/bin/
          '';

          meta = with pkgs.lib; {
            description = "A Rust library and CLI for parsing MTK logo images";
            homepage = "https://github.com/lopestom/mtklogo";
            license = licenses.asl20;
          };
        };
      });

      devShells = forAllSystems ({ pkgs, rustToolchain, ... }: {
        default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.rust-analyzer
          ];
        };
      });
    };
}
