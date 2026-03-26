# Container to quickly experiment with all nextless components.

# Uses the nix-ld package and mimics the behavior of the nix-ld nixos module-to allow for vscode remote connections.
# https://github.com/NixOS/nixpkgs/blob/1343204ea1bbb60fd93442d77d0798b02f3b9c7a/nixos/modules/programs/nix-ld.nix

# Nix devcontainer references
# https://valentinpratz.de/posts/2025-11-15-dev-tools-docker/
# https://github.com/NixOS/nixpkgs/issues/373143
# https://github.com/NixOS/nixpkgs/blob/ed142ab1b3a092c4d149245d0c4126a5d7ea00b0/nixos/modules/config/system-path.nix#L11

{pkgs, nextless_pkgs, toolchain, system}:
let
  # https://tech.aufomm.com/how-to-build-multi-arch-docker-image-on-nixos/#Create-multi-arch-image
  archSuffix = if system == "x86_64-linux" then "amd64" else "arm64";

  # VSCode remote requires this to work (with nix-ld).
  fake_nixos = pkgs.writeTextFile {
    name = "os-release";
    text = ''ID=nixos'';
    destination = "/etc/os-release";
  };

  # This also ensures the WorkingDir exists.
  dev_readme = pkgs.writeTextFile {
    name = "dev_readme";
    text = (builtins.readFile ./quickstart.md);
    destination = "/development/README.md";
  };

  cli_config = pkgs.writeTextFile {
    name = "cli.toml";
    text = ''
      controller_url = "http://127.0.0.1:7001"
    '';
    destination = "/development/cli.toml";
  };

  tmux_start = pkgs.writeTextFile {
    name = "playground_tmux.sh";
    text = (builtins.readFile ./playground_tmux.sh);
    destination = "/playground_tmux.sh";
  };

  # https://stackoverflow.com/a/76243160
  # https://nix.dev/tutorials/working-with-local-files.html#union-explicitly-include-files
  nextless_codebase = pkgs.stdenv.mkDerivation {
    name = "nextless_codebase";
    src = pkgs.lib.cleanSource ../../.;
    destination = "/nextless";
    postInstall = ''
      mkdir -p $out/nextless
      cp -r . $out/nextless
    '';
  };

  node_template = pkgs.writeText "node_template" (builtins.readFile ./nextless_node.template.toml);
  controller_template = pkgs.writeText "controller_template" (builtins.readFile ./nextless_controller.template.toml);

  node_config = pkgs.runCommand  "node.toml" {
    nativeBuildInputs = [pkgs.envsubst];
    destination = "/development/node.toml";
    NODE_ID = "00000000-0000-0000-0000-000000000001";
    AGENT_URL_ANNOUNCED = "http://127.0.0.1:7021";
    INVOCATION_URL_ANNOUNCED = "http://127.0.0.1:7022";
    CONTROLLER_URL = "http://127.0.0.1:7001";
  } ''
    mkdir -p $out/development;
    ${pkgs.envsubst}/bin/envsubst < ${node_template} > $out/development/node.toml
  '';

  controller_config = pkgs.runCommand  "controller.toml" {
    nativeBuildInputs = [pkgs.envsubst];
    destination = "/development/controller.toml";
    PLACEMENT_STRATEGY = "random";
  } ''
    mkdir -p $out/development;
    ${pkgs.envsubst}/bin/envsubst < ${controller_template} > $out/development/controller.toml
  '';

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
  tag = "latest-${archSuffix}";
  created = "now";
  config = {
    EntryPoint = ["/bin/bash"];
    Cmd = ["/playground_tmux.sh"];
    # https://github.com/nix-community/nix-ld/blob/main/README.md?plain=1#L115
    # https://github.com/NixOS/nixpkgs/blob/1343204ea1bbb60fd93442d77d0798b02f3b9c7a/nixos/modules/programs/nix-ld.nix#L16C13-L16C53
    Env = [
      ''NIX_LD=${pkgs.stdenv.cc.bintools.dynamicLinker}''
      ''NIX_LD_LIBRARY_PATH=${nix_ld_lib_path}''
      ''USER=root''
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
      curl
      glow

      cargo-generate
      toolchain
      dev_readme
      nextless_pkgs.nextless_node
      nextless_pkgs.nextless_cli
      nextless_pkgs.nextless_controller
      cli_config
      node_config
      controller_config
      tmux_start
      nextless_codebase
    ];
    postBuild = ''
      mkdir "$out/tmp"
    '';
  };
}
