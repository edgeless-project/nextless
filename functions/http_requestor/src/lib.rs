// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct RequestorFun;

edgeless_function::generate!(RequestorFun);

impl HttpRequestorAPI for RequestorFun {
    type EDGELESS_HTTP_REQUEST = edgeless_http::EdgelessHTTPRequest;
    type EDGELESS_HTTP_RESPONSE = edgeless_http::EdgelessHTTPResponse;
    
    fn handle_internal(encoded_message: &[u8]) {
        log::info!("HTTP_Requestor: 'Internal' called, MSG: {:?}", encoded_message);

        let res = call_http_out(
            &edgeless_http::EdgelessHTTPRequest {
                protocol: edgeless_http::EdgelessHTTPProtocol::HTTPS,
                host: "api.github.com:443".to_string(),
                headers: std::collections::HashMap::<String, String>::from([
                    ("Accept".to_string(), "application/vnd.github+json".to_string()),
                    ("User-Agent".to_string(), "edgeless".to_string()),
                ]),
                body: None,
                method: edgeless_http::EdgelessHTTPMethod::Get,
                path: "/users/raphaelhetzel/keys".to_string(),
            },
        );

        if let Ok(response) = res {
            log::info!("HTTP_requestor: {:?}", std::str::from_utf8(&response.body.unwrap()));
        }
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("HTTP_Requestor: 'Init' called");
        delayed_cast(5000, "self", "wakeup".as_bytes());
    }

    fn handle_stop() {
        log::info!("HTTP_Requestor: 'Stop' called");
    }
}
