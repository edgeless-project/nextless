// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub async fn init(
    spawner: embassy_executor::Spawner,
    timer: esp_hal::timer::timg::Timer,
    rng: esp_hal::rng::Rng,
    radio_clock_control: esp_hal::peripherals::RADIO_CLK,
    radio: esp_hal::peripherals::WIFI,
    agent: edgeless_embedded::agent::EmbeddedAgent,
) -> embassy_net::Stack<'static> {
    log::info!("Wifi Start");

    static ESP_INIT: static_cell::StaticCell<esp_wifi::EspWifiController<'static>> = static_cell::StaticCell::new();

    let tmp_wifi = esp_wifi::init(timer, rng, radio_clock_control);
    match tmp_wifi {
        Ok(_) => {}
        Err(e) => {
            log::info!("Wifi Err: {:?}", e)
        }
    }
    let init = ESP_INIT.init(tmp_wifi.unwrap());

    let wifi = radio;

    let (controller, interfaces) = esp_wifi::wifi::new(init, wifi).unwrap();

    let wifi_interface = interfaces.sta;

    let net_config = embassy_net::Config::dhcpv4(Default::default());

    static STACK_RESOURCES_RAW: static_cell::StaticCell<embassy_net::StackResources<3>> = static_cell::StaticCell::new();

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        STACK_RESOURCES_RAW.init_with(embassy_net::StackResources::<3>::new),
        1234,
    );

    spawner.spawn(connection(controller)).unwrap();
    spawner.spawn(net_task(runner)).unwrap();
    spawner.spawn(network_watchdog(stack, agent)).unwrap();

    stack
}

#[embassy_executor::task]
async fn network_watchdog(stack: embassy_net::Stack<'static>, mut agent: edgeless_embedded::agent::EmbeddedAgent) {
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
async fn connection(mut controller: esp_wifi::wifi::WifiController<'static>) {
    loop {
        if esp_wifi::wifi::wifi_state() == esp_wifi::wifi::WifiState::StaConnected {
            controller.wait_for_event(esp_wifi::wifi::WifiEvent::StaDisconnected).await;
            log::info!("Sta Disconnect");
            embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = esp_wifi::wifi::Configuration::Client(esp_wifi::wifi::ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
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
async fn net_task(mut runner: embassy_net::Runner<'static, esp_wifi::wifi::WifiDevice<'static>>) {
    runner.run().await;
}
