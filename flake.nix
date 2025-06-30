{
  description = "A tool to generate GitHub Actions runner JIT tokens for Caliptra FPGA runners";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        cred-tool = pkgs.rustPlatform.buildRustPackage {
          pname = "cred-tool";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };

      in
      {
        packages.default = cred-tool;

        apps.default = flake-utils.lib.mkApp {
          drv = cred-tool;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rustfmt
            clippy
          ];
        };
      });
}

