# Allow starting the Nextless Services (Node, Controller) as NixOS Services.

# Some used references about nix modules / services:
# https://wiki.nixos.org/wiki/NixOS_modules
# https://discourse.nixos.org/t/the-right-way-to-package-apps-with-flakes-and-install-them/57152
# https://stackoverflow.com/a/73377726
# https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/audio/spotifyd.nix
# https://github.com/NixOS/nixpkgs/pull/126247
# https://discourse.nixos.org/t/conditionally-set-attribute/28864/2
# https://stackoverflow.com/a/77527661
# https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/databases/influxdb2.nix
# https://github.com/NixOS/nixpkgs/blob/nixos-24.05/nixos/modules/services/finance/odoo.nix

{ lib, pkgs, config, nextless, ... }:
let
  nextless_pkgs = nextless.packages.${pkgs.system};
  toml = (pkgs.formats.toml { });
  resource_config = node_settings:
    ({ } // lib.optionalAttrs node_settings.resources.http_ingress.enable {
      http_ingress_url = node_settings.resources.http_ingress.listen_url;
      http_ingress_provider = "http-ingress-1";
    } // lib.optionalAttrs node_settings.resources.http_egress.enable {
      http_egress_provider = "http-egress-1";
    } // lib.optionalAttrs node_settings.resources.file_log.enable {
      file_log_provider = "file-log-1";
    } // lib.optionalAttrs node_settings.resources.led_matrix.enable {
      led_matrix = "led-matrix-1";
    });
  node_config = node_settings: {
    general = {
      node_id = node_settings.node_id;
      agent_url = node_settings.agent_url_listen;
      agent_url_announced = node_settings.agent_url_announced;
      invocation_url = node_settings.invocation_url_listen;
      invocation_url_announced = node_settings.invocation_url_announced;
      metrics_url = node_settings.metrics_url_listen;
      controller_url = node_settings.controller_url;
    };
    wasmtime_runtime = { enabled = true; };
    native_runtime = { enabled = true; };

    resources = resource_config node_settings;
  };
  controller_config = controller_settings: {
    controller_grpc_listen_url = controller_settings.controller_grpc_listen_url;
    controller_coap_listen_url = controller_settings.controller_coap_listen_url;
    placement_strategy = "random";
  };
  node_config_file = params: toml.generate "node.toml" (node_config params);
  controller_config_file = params:
    toml.generate "controller.toml" (controller_config params);
in {
  options = {
    services.nextless_node = {
      enable = lib.mkEnableOption "Nextless Worker Node";
      settings = {
        node_id = lib.mkOption {
          type = lib.types.str;
          default = "00000000-0000-0000-0000-000000000001";
        };
        node_labels = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
        };
        agent_url_listen = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7021";
        };
        agent_url_announced = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7021";
        };
        invocation_url_listen = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7022";
        };
        invocation_url_announced = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7022";
        };
        metrics_url_listen = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7023";
        };
        controller_url = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7001";
        };
        resources = {
          http_ingress = {
            enable = lib.mkEnableOption "Enable HTTP Ingress Resource";
            listen_url = lib.mkOption {
              type = lib.types.str;
              default = "http://0.0.0.0:7035";
            };
          };
          http_egress = {
            enable = lib.mkEnableOption "Enable HTTP Egress Resource";
          };
          file_log = {
            enable = lib.mkEnableOption "Enable File Log Resource";
          };
          led_matrix = {
            enable = lib.mkEnableOption "Enable LED Matrix Resource";
          };
        };
      };
    };
    services.nextless_controller = {
      enable = lib.mkEnableOption "Nextless Controller";
      settings = {
        controller_grpc_listen_url = lib.mkOption {
          type = lib.types.str;
          default = "http://127.0.0.1:7001";
        };
        controller_coap_listen_url = lib.mkOption {
          type = lib.types.str;
          default = "coap://127.0.0.1:7001";
        };
        prometheus = {
          enable = lib.mkEnableOption "Retrieve Telemetry from Prometheus.";
          url = lib.mkOption {
            type = lib.types.str;
            default = "http://127.0.0.1:9090";
          };
        };
      };
    };
  };

  config = lib.mkMerge [
    (lib.mkIf config.services.nextless_node.enable {
      users.users.nextless_node = {
        isSystemUser = true;
        group = "nextless_node";
        createHome = true;
        home = "/opt/nextless_node";
      };
      users.groups.nextless_node = { };
      systemd.services.nextless_node = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          ExecStart = "${nextless_pkgs.nextless_node}/bin/edgeless_node_d -c ${
              node_config_file config.services.nextless_node.settings
            }";
          User = "nextless_node";
          Group = "nextless_node";
          WorkingDirectory = "/opt/nextless_node";
        };

      };
    })
    (lib.mkIf config.services.nextless_controller.enable {
      users.users.nextless_controller = {
        isSystemUser = true;
        group = "nextless_controller";
        createHome = true;
        home = "/opt/nextless_controller";
      };
      users.groups.nextless_controller = { };
      systemd.services.nextless_controller = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          ExecStart =
            "${nextless_pkgs.nextless_controller}/bin/edgeless_con_d -c ${
              controller_config_file
              config.services.nextless_controller.settings
            }";
          User = "nextless_controller";
          Group = "nextless_controller";
          WorkingDirectory = "/opt/nextless_controller";
        };
      };
    })
  ];
}
