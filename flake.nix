{
  description = "lean-lsp";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.fenix = {
    url = "github:nix-community/fenix/monthly";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rust-toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-9se7PrPgIQRzVmopn9PtbQ292bfnFP+h/mpCFEHcgwY=";
        };
        rust-platform = pkgs.makeRustPlatform {
          rustc = rust-toolchain;
          cargo = rust-toolchain;
        };
      in
      {
        packages.default = rust-platform.buildRustPackage {
          pname = "lean-lsp";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;

            outputHashes = {
              "mkutils-0.1.0" = "sha256-LQ6T0SiKmsHFlVZaTiGA9zNXQH/eSJIOnqSMxtgbQp4=";
              "poem-3.1.12" = "sha256-UokXA76/PKGAp6NDKlKkT6wkxWdD8wxj50wPXyhn228=";
            };
          };

          RUSTFLAGS = "--cfg tokio_unstable --cfg tracing_unstable";

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
        };

        devShells.default = pkgs.mkShell {
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];

          packages = [
            pkgs.pkg-config
            pkgs.openssl
            pkgs.tokio-console

            rust-toolchain
          ];
        };
        formatter = pkgs.nixfmt;
      }
    );
}
