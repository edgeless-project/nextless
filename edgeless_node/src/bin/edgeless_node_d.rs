fn main() -> anyhow::Result<()> {
    env_logger::init();

    let node_id = uuid::Uuid::new_v4();
    let api_addr = "http://127.0.0.1:7001".to_string();

    let async_runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let mut async_tasks = vec![];

    let node_config = edgeless_node::EdgelessNodeSettings {
        node_id: node_id.clone(),
        agent_grpc_api_addr: api_addr.clone(),
    };

    async_tasks.push(async_runtime.spawn(edgeless_node::edgeless_node_main(node_config)));

    #[cfg(feature = "inabox")]
    {
        log::info!("Edgeless In A Box Mode");
        async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main()));
        let orc_config = edgeless_orc::EdgelessOrcSettings {
            nodes: vec![edgeless_orc::EdgelessOrcNodeConfig {
                node_id: node_id.clone(),
                api_addr: api_addr.clone(),
            }],
        };
        async_tasks.push(async_runtime.spawn(edgeless_orc::edgeless_orc_main(orc_config)));
        async_tasks.push(async_runtime.spawn(edgeless_con::edgeless_con_main()));
    }

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
