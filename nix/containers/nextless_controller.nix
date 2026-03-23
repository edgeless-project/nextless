# Basic Container for Nextless Node

{pkgs, nextless_pkgs}:
let
  nextless_controller_config = pkgs.writeTextFile {
    name = "nextless_controller.template.toml";
    text = (builtins.readFile ./nextless_controller.template.toml);
  };
in
pkgs.dockerTools.buildLayeredImage {
  name =  "nextless_controller";
  tag = "latest";
  created = "now";
  config = {
    # Use of envsubst inspired by:
    # https://github.com/edgeless-project/edgeless/blob/main/edgeless_node/Dockerfile#L56
    Env = [
      ''PLACEMENT_STRATEGY=random''
    ];
    ExposedPorts = {
      "7001" = {};
    };
    EntryPoint = [
      # https://stackoverflow.com/a/39270967
      "${pkgs.tini}/bin/tini"
      "--"
      "${pkgs.bash}/bin/bash"
      "-c"
      ''
        ${pkgs.envsubst}/bin/envsubst < ${nextless_controller_config} > /nextless_controller.toml && \
        ${nextless_pkgs.nextless_controller}/bin/edgeless_con_d -c /nextless_controller.toml
      ''
    ];
  };
}
