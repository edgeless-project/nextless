{ pkgs }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    vulkan-loader
    openssl.dev
    openssl
    pkg-config
    protobuf
    # mold
    gcc
    libgcc
    binaryen # wasm-opt
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
}
