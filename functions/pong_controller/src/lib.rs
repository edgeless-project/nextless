// SPDX-FileCopyrightText: © 2026 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub mod configuration;
pub mod game_state;
pub mod messages;

use edgeless_function::*;

const PADDLE_LEN: u64 = 12;
const PADDLE_WIDTH: u64 = 4;
const BALL_SIZE: u64 = 4;

struct PongController;

edgeless_function::generate!(PongController);

#[derive(Debug, serde::Deserialize)]
struct ControllerInput {
    id: usize,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
    y: bool,
    x: bool,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<game_state::GameState>> = std::sync::OnceLock::new();

impl PongControllerAPI<'_> for PongController {
    type EFT_HTTP_REQUEST = edgeless_function_types::http::EdgelessHTTPRequest;
    type EFT_HTTP_RESPONSE = edgeless_function_types::http::EdgelessHTTPResponse;
    type PONG_RENDER_REQUEST = messages::PongRenderRequest;

    fn handle_call_user_input(_src: InstanceId, req: Self::EFT_HTTP_REQUEST) -> Self::EFT_HTTP_RESPONSE {
        let mut game_state = STATE.get().unwrap().lock().unwrap();

        if req.path == "/index.html" || req.path == "" || req.path == "/" {
            // This file is not included on github as it contains AI-generated content.
            let html = include_bytes!("../data/controller.html");

            edgeless_function_types::http::EdgelessHTTPResponse {
                status: 200,
                body: Some(html.to_vec()),
                headers: std::collections::HashMap::<String, String>::from([("Content-Type".to_string(), "text/html".to_string())]),
            }
        } else if req.path == "/controller_input" {
            if let Some(body) = req.body {
                let body_str = String::from_utf8(body).unwrap();

                let input: ControllerInput = serde_json::from_str(&body_str).unwrap();

                log::trace!("Input: {input:?}");

                if input.down {
                    game_state.move_paddle_down(input.id);
                }

                if input.up {
                    game_state.move_paddle_up(input.id);
                }

                // Use the second side of the controller for the second paddle (please don't cheat).
                if input.y {
                    game_state.move_paddle_up(if input.id == 1 { 2 } else { 1 });
                }
                if input.a {
                    game_state.move_paddle_down(if input.id == 1 { 2 } else { 1 });
                }
            }

            edgeless_function_types::http::EdgelessHTTPResponse {
                status: 200,
                body: Some(Vec::<u8>::from("")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        } else {
            edgeless_function_types::http::EdgelessHTTPResponse {
                status: 404,
                body: Some(Vec::<u8>::from("Not Found")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        }
    }

    fn handle_internal(_data: &[u8]) {
        let mut game_state = STATE.get().unwrap().lock().unwrap();

        game_state.logic_update();

        let message = game_state.as_render_request();
        cast_render(&message);

        delayed_cast(30 as u64, "self", &1u64.to_le_bytes());
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Started.");

        let payload_str = payload.map(|p| str::from_utf8(p).unwrap()).unwrap_or("");
        let config = configuration::Configuration::parse(payload_str);

        STATE
            .set(std::sync::Mutex::new(game_state::GameState::new(config.size_y, config.size_x)))
            .unwrap();

        delayed_cast(30 as u64, "self", &1u64.to_le_bytes());
    }

    fn handle_stop() {
        log::info!("Stopped.");
    }
}
