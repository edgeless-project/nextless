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
        nextless_node_packages = (import ./nix/packages/nextless_node.nix) {inherit pkgs toolchain;};
      in {
        packages = {
          nextless_cli = (import ./nix/packages/nextless_cli.nix) {inherit pkgs toolchain;};
          nextless_node = nextless_node_packages.nextless_node;
          nextless_node_led_matrix = nextless_node_packages.nextless_node_led_matrix;
          nextless_controller = (import ./nix/packages/nextless_controller.nix) {inherit pkgs toolchain;};
        };

        devShell = (import ./nix/devshell.nix) {inherit pkgs;};
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
    ) //
    {
      nixosModules = {
        nextless = ./nix/nixos_module.nix;
        default = self.nixosModules.nextless; # disko states this is a convention.
      };
    }
    ;
}
