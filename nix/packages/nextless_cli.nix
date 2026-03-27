{ pkgs, toolchain }:
(pkgs.makeRustPlatform {
  cargo = toolchain;
  rustc = toolchain;
}).buildRustPackage
  rec {
    pname = "edgeless_cli";
    version = "3.0.1";
    doCheck = false;
    buildAndTestSubdir = "edgeless_cli";
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
      openssl
      toolchain
      makeWrapper
      gcc
      binaryen
    ];
    postInstall = ''
      wrapProgram $out/bin/edgeless_cli \
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
