{ pkgs, toolchain }:
(pkgs.makeRustPlatform {
  cargo = toolchain;
  rustc = toolchain;
}).buildRustPackage
  rec {
    pname = "edgeless_con_d";
    version = "0.1";
    doCheck = false;
    buildAndTestSubdir = "edgeless_con";
    cargoLock = {
      lockFile = ../../Cargo.lock;
      allowBuiltinFetchGit = true;
    };
    src = pkgs.lib.cleanSource ../../.;
    nativeBuildInputs = with pkgs; [
      openssl.dev
      perl
      pkg-config
      protobuf
    ];
    buildInputs = with pkgs; [
      openssl # TODO Unify OpenSSL Usage
      toolchain
      makeWrapper
      gcc
      binaryen
    ];
    # https://discourse.nixos.org/t/program-compiled-with-rust-cannot-find-libssl-so-3-at-runtime/27196
    postInstall = ''
      wrapProgram $out/bin/edgeless_con_d \
        --set PATH ${
          pkgs.lib.makeBinPath [
            toolchain
            pkgs.gcc
            pkgs.binaryen
          ]
        } \
        --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath [ pkgs.openssl ]}
    '';
  }
