inputs@{
  flake-parts,
  systems,
  rust-overlay,
  treefmt-nix,
  ...
}:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = import systems;

  imports = [ treefmt-nix.flakeModule ];

  perSystem =
    {
      config,
      pkgs,
      ...
    }:
    let
      rootSrc = ./..;
      cargoToml = builtins.fromTOML (builtins.readFile (rootSrc + "/Cargo.toml"));
      packageName = cargoToml.package.name;
      packageVersion = cargoToml.package.version;

      rustPkgs = pkgs.extend rust-overlay.overlays.default;
      rustToolchain = rustPkgs.rust-bin.fromRustupToolchainFile (rootSrc + "/rust-toolchain.toml");
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };

      rustPackage = rustPlatform.buildRustPackage {
        pname = packageName;
        version = packageVersion;
        src = rootSrc;
        cargoLock.lockFile = rootSrc + "/Cargo.lock";

        doCheck = false;
      };

      rustDevPackages = [
        rustToolchain
        pkgs.rust-analyzer
        pkgs.cargo-watch
        pkgs.cargo-edit
        pkgs.pkg-config
        pkgs.kitty
      ];
    in
    {
      treefmt = {
        projectRootFile = "flake.nix";
        programs.rustfmt.enable = true;
        programs.nixfmt.enable = true;
      };

      packages = {
        rust = rustPackage;
        default = rustPackage;
      };

      checks = {
        build = rustPackage;
      };

      apps.kitty-runner = {
        type = "app";
        program = "${rustPackage}/bin/kitty-runner";
      };

      devShells = {
        rust = pkgs.mkShell {
          packages = rustDevPackages;
        };

        default = pkgs.mkShell {
          packages = rustDevPackages ++ [ config.treefmt.build.wrapper ];
        };
      };

      formatter = pkgs.nixfmt-rfc-style;
    };
}
