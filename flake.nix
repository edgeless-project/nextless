{
  description = "A basic flake for the EU EDGELESS project's MVP";

  nixConfig = {
    extra-substituters = [
      "https://edgeless.cachix.org"
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "edgeless.cachix.org-1:CpGxIJOpDU+VmvGJSnSPCTFZ1rytGUFc/x7Op9T8t0I="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = with fenix.packages.${system}; combine [
          latest.toolchain
          targets.wasm32-unknown-unknown.latest.rust-std
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
            # mold
            gcc
            binaryen #wasm-opt
            curl # libcurl used in the cli. Not sure why it is not needed in the CLI.
            # While i would prefer to use fenix here, we depend on the ESP toolchain and rust-toolchain.toml
            # which are specific to rustup / espup.
            # toolchain
            rustup
            espup
            espflash
            binutils
            lld
            (pkgs.python3.withPackages (pypkg: [
              pypkg.pandas
              pypkg.scapy
              pypkg.seaborn
            ]))
          ];
          shellHook = ''
            # rustup install stable
            # espup install
            source ~/export-esp.sh
          '';
        };
      }
    );
}
