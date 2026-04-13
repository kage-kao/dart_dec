{
  description = "dart_dec — Dart AOT Headless Decompiler";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
          sqlite
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ];

      in {
        # Build the dart_dec binary
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "dart_dec";
          version = "0.1.0";
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Only build the CLI binary
          cargoBuildFlags = [ "-p" "dart_dec_cli" ];

          meta = with pkgs.lib; {
            description = "Dart AOT Headless Decompiler";
            homepage = "https://github.com/dart-dec/dart_dec";
            mainProgram = "dart_dec";
          };
        };

        # Build the Python bindings
        packages.python = pkgs.python3Packages.buildPythonPackage {
          pname = "dart_dec";
          version = "0.1.0";
          src = self;

          format = "other";

          nativeBuildInputs = nativeBuildInputs ++ [
            pkgs.python3Packages.maturin
          ];

          inherit buildInputs;

          buildPhase = ''
            cd crates/dart_dec_python
            maturin build --release
          '';

          installPhase = ''
            pip install target/wheels/dart_dec-*.whl --prefix=$out
          '';
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            echo "dart_dec development environment"
            echo "Build: cargo build --release"
            echo "Test:  cargo test --workspace"
            echo "Bench: cargo bench"
          '';
        };

        # Docker image
        packages.docker = pkgs.dockerTools.buildImage {
          name = "dart_dec";
          tag = "latest";
          copyToRoot = [ self.packages.${system}.default ];
          config = {
            Entrypoint = [ "/bin/dart_dec" ];
          };
        };
      }
    );
}
