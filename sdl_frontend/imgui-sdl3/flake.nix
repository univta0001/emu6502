{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, fenix, utils, nixpkgs }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        toolchain = with fenix.packages.${system}; fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
        };

      in {
        devShell = with pkgs; mkShell {
          buildInputs = [
            # rust
            toolchain
            
            # sdl3
            sdl3
            
            # shaderc
            cmake stdenv.cc.cc

            # tools
            mdbook just typos cargo-deny taplo
            llvmPackages_21.clang-tools # clang-format
            git-cliff
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath [ sdl3 stdenv.cc.cc ]}";
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          RUSTFLAGS = "-Awarnings";
        };
      }
    );
}
