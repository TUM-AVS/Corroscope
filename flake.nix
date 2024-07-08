{
  description = "Corroscope Scenario Viewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;


        craneLib = (crane.mkLib pkgs).overrideToolchain
          (fenix.packages.${system}.latest.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
            "rust-src"
            # "clippy"
          ]);

        # craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.mold
            pkgs.clang
          ];

          buildInputs = [
            # Add additional build inputs here
            pkgs.protobuf
            pkgs.wayland
            pkgs.xorg.libX11
            pkgs.libGL
            pkgs.vulkan-loader
            pkgs.libxkbcommon
          ];
        };

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        corroscope = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit corroscope;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          corroscope-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          corroscope-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          corroscope-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          corroscope-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          corroscope-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `corroscope` if you do not want
          # the tests to run twice
          corroscope-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        };

        packages = {
          default = corroscope;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          corroscope-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = corroscope;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXrandr
            pkgs.xorg.libXi
            pkgs.libGL
            pkgs.vulkan-loader
            pkgs.vulkan-validation-layers
            pkgs.libxkbcommon
            pkgs.libgcc.lib
          ];

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = 
            let
              pkgs = import nixpkgs { inherit system; overlays = [ fenix.overlays.default ]; };
            in [
            pkgs.bloaty
            pkgs.tracy

            pkgs.rust-analyzer-nightly
          ];
        };
      });
}
