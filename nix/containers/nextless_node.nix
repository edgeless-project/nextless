# Basic Container for Nextless Node

# References about nix-based containers:
# https://thewagner.net/blog/2021/02/25/building-container-images-with-nix/
# https://nixos.org/manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage
# https://www.syseleven.de/blog/deklarative-reproduzierbare-und-sichere-oci-container-mit-nix/
# https://tech.aufomm.com/how-to-build-multi-arch-docker-image-on-nixos/

{pkgs, nextless_pkgs}:
let
  nextless_node_config_template = pkgs.writeTextFile {
    name = "nextless_node.template.toml";
    text = (builtins.readFile ./nextless_node.template.toml);
  };
in
pkgs.dockerTools.buildLayeredImage {
  name =  "nextless_node";
  tag = "latest";
  created = "now";
  config = {
    # Use of envsubst inspired by:
    # https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/Dockerfile#L56
    Env = [
      ''NODE_ID=00000000-0000-0000-0000-000000000001''
      ''AGENT_URL_ANNOUNCED=http://127.0.0.1:7021''
      ''INVOCATION_URL_ANNOUNCED=http://127.0.0.1:7022''
      ''CONTROLLER_URL=http://127.0.0.1:7001''
    ];
    ExposedPorts = {
      "7021" = {};
      "7022" = {};
      "7023" = {};
      "7035" = {};
    };
    EntryPoint = [
      # https://stackoverflow.com/a/39270967
      "${pkgs.tini}/bin/tini"
      "--"
      "${pkgs.bash}/bin/bash"
      "-c"
      ''
        ${pkgs.envsubst}/bin/envsubst < ${nextless_node_config_template} > /nextless_node.toml && \
        ${nextless_pkgs.nextless_node}/bin/edgeless_node_d -c /nextless_node.toml
      ''
    ];
  };
}
