{
  description = "Kitty test harness for driving kitty terminal via remote control";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, systems, rust-overlay, crane, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import systems;

      perSystem = { system, ... }:
        let
          overlays = [ rust-overlay.overlays.default ];
          pkgs = import nixpkgs { inherit system overlays; };
          rustToolchain = pkgs.rust-bin.nightly.latest.default;
          craneLib = (crane.lib.${system}).overrideToolchain rustToolchain;
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            pname = "kitty-test-harness";
            version = "0.1.0";
            inherit src;
            cargoLock = ./Cargo.lock;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          packages.default = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });

          apps.kitty-runner = {
            type = "app";
            program = "${self.packages.${system}.default}/bin/kitty-runner";
          };

          devShells.default = craneLib.devShell {
            inherit cargoArtifacts;
            packages = with pkgs; [
              rustToolchain
              rust-analyzer
              kitty
              pkg-config
            ];
          };

          formatter = pkgs.nixfmt-rfc-style;
        };
    };
}
