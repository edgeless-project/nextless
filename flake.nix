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
          minimal.toolchain
          targets.wasm32-unknown-unknown.latest.rust-std
        ];
        node_package = {
          pname = "edgeless_node_d";
          version = "0.1";
          doCheck = false;
          buildAndTestSubdir = "edgeless_node";
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          src = pkgs.lib.cleanSource ./.;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            openssl.dev
            vulkan-loader
            pkg-config
            protobuf
            makeWrapper
          ];
          buildInputs = with pkgs; [
            openssl
            vulkan-loader
          ];
          postInstall = ''
            wrapProgram $out/bin/edgeless_node_d \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath [pkgs.openssl pkgs.vulkan-loader]}
          '';
        };
      in {
        packages = {
          nextless_cli = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage rec {
            pname = "edgeless_cli";
            version = "3.0.1";
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
              openssl
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
                ]} \
                --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath [pkgs.openssl]}
            '';
          };
          nextless_node = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage node_package;
          nextless_node_led_matrix = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage (node_package // {
            buildFeatures = [ "hardware_led_matrix" ];
          });
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
              # https://discourse.nixos.org/t/program-compiled-with-rust-cannot-find-libssl-so-3-at-runtime/27196
            postInstall = ''
              wrapProgram $out/bin/edgeless_con_d \
                --set PATH ${pkgs.lib.makeBinPath [toolchain pkgs.gcc pkgs.binaryen]} \
                --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath [pkgs.openssl]}
            '';
            };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            vulkan-loader
            openssl.dev
            openssl
            pkg-config
            protobuf
            # mold
            gcc
            libgcc
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
            SDL2
            (pkgs.python3.withPackages (pypkg: [
              pypkg.jupyter
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
    ) //
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
    ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        nextless_pkgs = self.packages.${system};
        toolchain = with fenix.packages.${system}; combine [
          minimal.toolchain
          targets.wasm32-unknown-unknown.latest.rust-std
        ];
      in
      {
        containers = {
          nextless_node = (import ./nix/containers/nextless_node.nix) {inherit pkgs nextless_pkgs;};
          nextless_controller = (import ./nix/containers/nextless_controller.nix) {inherit pkgs nextless_pkgs;};
          nextless_playground = (import ./nix/containers/nextless_playground.nix) {inherit pkgs nextless_pkgs toolchain system;};
        };
      }
    );
}
