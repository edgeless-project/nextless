pub fn edgeless_inabox_main(
    async_runtime: &tokio::runtime::Runtime,
    async_tasks: &mut Vec<tokio::task::JoinHandle<()>>,
    node_conf_file: &str,
    orc_conf_file: &str,
    bal_conf_file: &str,
    con_conf_file: &str,
) -> anyhow::Result<()> {
    let node_conf: edgeless_node::EdgelessNodeSettings = toml::from_str(&std::fs::read_to_string(node_conf_file)?)?;
    let orc_conf: edgeless_orc::EdgelessOrcSettings = toml::from_str(&std::fs::read_to_string(orc_conf_file)?)?;
    let bal_conf: edgeless_bal::EdgelessBalSettings = toml::from_str(&std::fs::read_to_string(bal_conf_file)?)?;
    let con_conf: edgeless_con::EdgelessConSettings = toml::from_str(&std::fs::read_to_string(con_conf_file)?)?;

    log::info!("Edgeless In A Box");

    async_tasks.push(async_runtime.spawn(edgeless_node::edgeless_node_main(node_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main(bal_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_orc::edgeless_orc_main(orc_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_con::edgeless_con_main(con_conf.clone())));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start() -> anyhow::Result<()> {
        // create default configuration files
        let mut dir = std::env::temp_dir();
        dir.push("test_start_remove_me");
        println!("temp dir: {:?}", dir);
        if dir.exists() {
            std::fs::remove_dir_all(dir.to_str().unwrap())?;
        }
        std::fs::create_dir_all(dir.to_str().unwrap())?;
        let node_conf = dir.join(std::path::Path::new("node.toml")).to_str().unwrap().to_string();
        let orc_conf = dir.join(std::path::Path::new("orchestrator.toml")).to_str().unwrap().to_string();
        let bal_conf = dir.join(std::path::Path::new("balancer.toml")).to_str().unwrap().to_string();
        let con_conf = dir.join(std::path::Path::new("controller.toml")).to_str().unwrap().to_string();
        println!("node conf: {}", node_conf);
        println!("orc  conf: {}", orc_conf);
        println!("bal  conf: {}", bal_conf);
        println!("con  conf: {}", con_conf);
        edgeless_api::util::create_template(node_conf.as_str(), edgeless_node::edgeless_node_default_conf().as_str())?;
        edgeless_api::util::create_template(orc_conf.as_str(), edgeless_orc::edgeless_orc_default_conf().as_str())?;
        edgeless_api::util::create_template(bal_conf.as_str(), edgeless_bal::edgeless_bal_default_conf().as_str())?;
        edgeless_api::util::create_template(con_conf.as_str(), edgeless_con::edgeless_con_default_conf().as_str())?;

        // start the services, terminate soon after
        let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
        let mut async_tasks = vec![];

        edgeless_inabox_main(
            &async_runtime,
            &mut async_tasks,
            node_conf.as_str(),
            orc_conf.as_str(),
            bal_conf.as_str(),
            con_conf.as_str(),
        )?;

        std::thread::sleep(std::time::Duration::from_millis(500));
        async_tasks.clear();

        // clean up test artifacts
        std::fs::remove_dir_all(dir)?;

        Ok(())
    }
}