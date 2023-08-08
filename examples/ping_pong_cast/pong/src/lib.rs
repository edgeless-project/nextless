use edgeless_function::api::*;
struct PongerFun;

impl Edgefunction for PongerFun {
    fn handle_cast(src: Fid, encoded_message: String) {
        log::info!("AsyncPonger: 'Cast' called, MSG: {}", encoded_message);
        cast(&src, "PONG");
        // OR: 
        // cast_alias("pinger", "PONG2");
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        log::info!("AsyncPonger: 'Call' called, MSG: {}", encoded_message);
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("AsyncPonger: 'Init' called");
    }

    fn handle_stop() {
        log::info!("AsyncPonger: 'Stop' called");
    }
}
edgeless_function::export!(PongerFun);
