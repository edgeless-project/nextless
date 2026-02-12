// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub async fn init(
    spawner: embassy_executor::Spawner,
    rng: esp_hal::rng::Rng,
    radio: esp_hal::peripherals::WIFI<'static>,
    agent: edgeless_embedded::agent::EmbeddedAgent,
) -> embassy_net::Stack<'static> {
    log::info!("Wifi Start");

    static ESP_INIT: static_cell::StaticCell<esp_radio::Controller<'static>> = static_cell::StaticCell::new();

    let tmp_wifi = esp_radio::init();
    match tmp_wifi {
        Ok(_) => {}
        Err(e) => {
            log::info!("Wifi Err: {:?}", e)
        }
    }
    let init = ESP_INIT.init(tmp_wifi.unwrap());

    #[cfg(feature = "esp32")]
    let (controller, interfaces) = esp_radio::wifi::new(
        init,
        radio,
        esp_radio::wifi::Config::default()
            .with_country_code(*b"DE")
            .with_rx_queue_size(5)
            .with_tx_queue_size(3)
            .with_static_rx_buf_num(8)
            .with_dynamic_rx_buf_num(8)
            .with_static_tx_buf_num(0)
            .with_dynamic_tx_buf_num(8)
            .with_ampdu_rx_enable(false)
            .with_ampdu_tx_enable(false)
            .with_amsdu_tx_enable(false),
    )
    .unwrap();

    #[cfg(feature = "esp32s3")]
    let (controller, interfaces) = esp_radio::wifi::new(init, radio, esp_radio::wifi::Config::default().with_country_code(*b"DE")).unwrap();

    log::info!("Radio Initialized");

    let wifi_interface = interfaces.sta;

    let net_config = embassy_net::Config::dhcpv4(Default::default());

    static STACK_RESOURCES_RAW: static_cell::StaticCell<embassy_net::StackResources<3>> = static_cell::StaticCell::new();

    let rng = esp_hal::rng::Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        STACK_RESOURCES_RAW.init_with(embassy_net::StackResources::<3>::new),
        seed,
    );

    spawner.spawn(connection(controller)).unwrap();
    spawner.spawn(net_task(runner)).unwrap();
    spawner.spawn(network_watchdog(stack, agent)).unwrap();

    stack
}

#[embassy_executor::task]
async fn network_watchdog(stack: embassy_net::Stack<'static>, mut agent: edgeless_embedded::agent::EmbeddedAgent) {
    log::info!("Start Network Watchdog");
    loop {
        if stack.is_link_up() {
            break;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}. Registering with the Orchestrator.", config.address);
            agent.register(config.address.address()).await;
            log::info!("Registered with the Orchestrator.");
            break;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }

    loop {
        if !stack.is_link_up() {
            log::info!("Link is Down");
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(1000)).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) {
    loop {
        if esp_radio::wifi::sta_state() == esp_radio::wifi::WifiStaState::Connected {
            controller.wait_for_event(esp_radio::wifi::WifiEvent::StaDisconnected).await;
            log::info!("Sta Disconnect");
            embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            log::info!("Wifi Controller Started");
            let client_config = esp_radio::wifi::ModeConfig::Client(
                esp_radio::wifi::ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            log::info!("Starting wifi. SSID: {}", SSID);
            controller.start_async().await.unwrap();
        }

        log::info!("Attempt to connect.");
        match controller.connect_async().await {
            Ok(_) => log::info!("Wifi connected!"),
            Err(e) => {
                log::error!("Failed to connect to wifi: {e:?}");
                embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, esp_radio::wifi::WifiDevice<'static>>) {
    runner.run().await;
}
