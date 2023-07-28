use futures::join;

pub mod agent;
pub mod runner_api;
pub mod rust_runner;
pub mod state_management;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessNodeSettings {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
    pub invocation_url: String,
    pub peers: Vec<edgeless_dataplane::EdgelessDataplaneSettingsPeer>,
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);
    let state_manager = state_management::StateManager::new().await;
    let data_plane =
        edgeless_dataplane::DataPlaneChainProvider::new(settings.node_id.clone(), settings.invocation_url.clone(), settings.peers.clone()).await;
    let (mut rust_runner, rust_runner_task) = rust_runner::Runner::new(settings.clone(), data_plane.clone(), state_manager.clone());
    let (mut agent, agent_task) = agent::Agent::new(rust_runner.get_api_client(), settings.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url);

    join!(rust_runner_task, agent_task, agent_api_server);
}

pub fn edgeless_node_default_conf() -> String {
    String::from(
        r##"node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7001"
invocation_url = "http://127.0.0.1:7002"
peers = [
        {id = "fda6ce79-46df-4f96-a0d2-456f720f606c", invocation_url="http://127.0.0.1:7002" },
        {id = "2bb0867f-e9ee-4a3a-8872-dbaa5228ee23", invocation_url="http://127.0.0.1:7032" }
]
"##,
    )
}
