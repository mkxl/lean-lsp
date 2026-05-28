{
  description = "lean-lsp";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.fenix = {
    url = "github:nix-community/fenix/monthly";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.crane.url = "github:ipetkov/crane";

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        rust-toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-fx771dMiW4FXGenjzuC1dpm4R4qZa037EVRBDPsp/Zg=";
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain (pkgs: rust-toolchain);
      in
      {
        packages.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
        };

        devShells.default = craneLib.devShell {
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.cargo-watch
            pkgs.tokio-console
          ];
        };

        formatter = pkgs.nixfmt;
      }
    );
}
