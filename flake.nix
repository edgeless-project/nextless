{
  description = "A basic flake for the EU EDGELESS project's MVP";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = with fenix.packages.${system}; combine [
          stable.toolchain
          targets.wasm32-unknown-unknown.stable.rust-std
        ];
      in {
        packages = {
          nextless_cli = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage rec {
            pname = "edgeless_cli";
            version = "0.1";
            doCheck = false;
            buildAndTestSubdir = "edgeless_cli";
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = with pkgs; [
              openssl.dev
              perl
              pkg-config
              protobuf
            ];
            buildInputs = with pkgs; [
              toolchain
              makeWrapper
              gcc
              binaryen
            ];
            postInstall = ''
              wrapProgram $out/bin/edgeless_cli \
                --set PATH ${pkgs.lib.makeBinPath [
                  toolchain
                  pkgs.gcc
                  pkgs.binaryen
                ]}
            '';
          };
          nextless_node = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage rec {
            pname = "edgeless_node_d";
            version = "0.1";
            doCheck = false;
            buildAndTestSubdir = "edgeless_node";
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = with pkgs; [
              openssl.dev
              perl
              pkg-config
              protobuf
            ];
          };
          nextless_controller = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage rec {
            pname = "edgeless_con_d";
            version = "0.1";
            doCheck = false;
            buildAndTestSubdir = "edgeless_con";
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = with pkgs; [
              openssl.dev
              perl
              pkg-config
              protobuf
            ];
            buildInputs = with pkgs; [
              openssl #TODO Unify OpenSSL Usage
              toolchain
              makeWrapper
              gcc
              binaryen
            ];
            postInstall = ''
              wrapProgram $out/bin/edgeless_con_d \
                --set PATH ${pkgs.lib.makeBinPath [
                  toolchain
                  pkgs.gcc
                  pkgs.binaryen
                ]}
            '';
            };
        };


        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            openssl.dev
            pkg-config
            protobuf
            toolchain
            mold
            gcc
            binaryen
          ];
        };
      }
    );
}
