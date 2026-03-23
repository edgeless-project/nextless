# Container to quickly experiment with all nextless components.

# Uses the nix-ld package and mimics the behavior of the nix-ld nixos module-to allow for vscode remote connections.
# https://github.com/NixOS/nixpkgs/blob/1343204ea1bbb60fd93442d77d0798b02f3b9c7a/nixos/modules/programs/nix-ld.nix

# Nix devcontainer references
# https://valentinpratz.de/posts/2025-11-15-dev-tools-docker/
# https://github.com/NixOS/nixpkgs/issues/373143
#

{pkgs, nextless_pkgs, toolchain}:
let
  # VSCode remote requires this to work (with nix-ld).
  fake_nixos = pkgs.writeTextFile {
    name = "os-release";
    text = ''ID=nixos'';
    destination = "/etc/os-release";
  };

  # This also ensures the WorkingDir exists.
  dev_readme = pkgs.writeTextFile {
    name = "dev_readme";
    text = ''This is a Development Environment.'';
    destination = "/development/README.txt";
  };

  # Helpers for nix_ld_symlink. This is replicating the behavior of nix-ld on nix-os.
  # Copied from https://github.com/NixOS/nixpkgs/blob/nixos-25.11/nixos/modules/config/ldso.nix#L17
  libDir = pkgs.stdenv.hostPlatform.libDir;
  ldsoBasename = builtins.unsafeDiscardStringContext (
    pkgs.lib.last (pkgs.lib.splitString "/" pkgs.stdenv.cc.bintools.dynamicLinker)
  );

  # Set up the symlink as required by nix-ld
  # https://github.com/NixOS/nixpkgs/blob/1343204ea1bbb60fd93442d77d0798b02f3b9c7a/nixos/modules/programs/nix-ld.nix#L36
  nix_ld_symlink = pkgs.buildEnv {
    name = "nix_ld_symlink";
    pathsToLink = [ "/${libDir}"];
    paths = map pkgs.lib.getLib [pkgs.nix-ld];
    postBuild = ''
      ln -s "${pkgs.nix-ld}/libexec/nix-ld" "$out/${libDir}/${ldsoBasename}"
    '';
  };

  nix_ld_lib_path = pkgs.lib.makeLibraryPath [
    pkgs.stdenv.cc.cc
    pkgs.glibc
    pkgs.libgcc
  ];
in
pkgs.dockerTools.buildImage {
  name = "nextless_playground";
  tag = "latest";
  created = "now";
  config = {
    Cmd = ["/bin/bash"];
    # https://github.com/nix-community/nix-ld/blob/main/README.md?plain=1#L115
    # https://github.com/NixOS/nixpkgs/blob/1343204ea1bbb60fd93442d77d0798b02f3b9c7a/nixos/modules/programs/nix-ld.nix#L16C13-L16C53
    Env = [
      ''NIX_LD=${pkgs.stdenv.cc.bintools.dynamicLinker}''
      ''NIX_LD_LIBRARY_PATH=${nix_ld_lib_path}''
    ];
    WorkingDir = "/development";
  };
  copyToRoot = pkgs.buildEnv {
    name = "playground_image_root";
    paths = with pkgs; [
      coreutils-full
      busybox
      getconf
      dockerTools.usrBinEnv
      dockerTools.fakeNss
      dockerTools.caCertificates
      stdenv

      nix-ld
      fake_nixos
      nix_ld_symlink

      bashInteractive
      ncurses
      tmux
      vim
      nano

      cargo-generate
      toolchain
      dev_readme
      nextless_pkgs.nextless_node
      nextless_pkgs.nextless_cli
      nextless_pkgs.nextless_controller
    ];
    postBuild = ''
      mkdir "$out/tmp"
    '';
  };
}
