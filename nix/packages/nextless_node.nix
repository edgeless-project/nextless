{ pkgs, toolchain }:
let
  node_package_config_base = {
    pname = "edgeless_node_d";
    version = "0.1";
    doCheck = false;
    buildAndTestSubdir = "edgeless_node";
    cargoLock = {
      lockFile = ../../Cargo.lock;
      allowBuiltinFetchGit = true;
    };
    src = pkgs.lib.cleanSource ../../.;
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
        --prefix LD_LIBRARY_PATH : ${
          pkgs.lib.makeLibraryPath [
            pkgs.openssl
            pkgs.vulkan-loader
          ]
        }
    '';
  };
in
{
  nextless_node =
    (pkgs.makeRustPlatform {
      cargo = toolchain;
      rustc = toolchain;
    }).buildRustPackage
      node_package_config_base;
  nextless_node_led_matrix =
    (pkgs.makeRustPlatform {
      cargo = toolchain;
      rustc = toolchain;
    }).buildRustPackage
      (
        node_package_config_base
        // {
          buildFeatures = [ "hardware_led_matrix" ];
        }
      );
}
