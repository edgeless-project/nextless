// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-hal/blob/main/esp32-hal/examples/embassy_hello_world.rs, https://github.com/esp-rs/esp-template/blob/main/src/main.rs & https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

#![no_std]
#![no_main]
#![feature(stmt_expr_attributes)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

#[cfg(feature = "epaper_2_13")]
pub mod epaper_display_impl;
#[cfg(feature = "scd30")]
pub mod scd30_sensor_impl;
#[cfg(feature = "scd30")]
use embedded_hal::delay::DelayNs;
pub mod wifi;

#[cfg(feature = "epaper_2_13")]
use edgeless_embedded::resource::epaper_display::EPaper;
#[cfg(feature = "epaper_2_13")]
use epd_waveshare::prelude::*;
use esp_alloc as _;
use esp_backtrace as _;

static RNG: once_cell::sync::OnceCell<esp_hal::rng::Rng> = once_cell::sync::OnceCell::new();

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

static mut APP_CORE_STACK: esp_hal::system::Stack<16384> = esp_hal::system::Stack::new();

// Originally was planning to use a dedicated heap here, but this is currently not possible: https://github.com/esp-rs/esp-hal/issues/3187
#[no_mangle]
pub extern "C" fn esp_wifi_free_internal_heap() -> usize {
    // return size of free allocatable RAM
    esp_alloc::HEAP.free_caps(esp_alloc::MemoryCapability::Internal.into())
}

#[no_mangle]
pub extern "C" fn esp_wifi_allocate_from_internal_ram(size: usize) -> *mut u8 {
    // allocate memory of size `size` from internal memory
    unsafe {
        esp_alloc::HEAP.alloc_caps(
            esp_alloc::MemoryCapability::Internal.into(),
            core::alloc::Layout::from_size_align_unchecked(size, 4),
        )
    }
}

#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom::Error> {
    match RNG.get() {
        Some(rng) => {
            let mut rng = *rng;
            for i in 0..len {
                *(dest.add(i)) = rng.random() as u8;
            }
            Ok(())
        }
        None => Err(getrandom::Error::UNSUPPORTED),
    }
}

#[esp_hal::main]
fn main() -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Debug);
    esp_println::println!("Start Edgeless Embedded.");

    let mut config = esp_hal::Config::default();
    config = config.with_cpu_clock(esp_hal::clock::CpuClock::max());
    #[cfg(feature = "psram")]
    {
        config = config.with_psram(esp_hal::psram::PsramConfig { ..Default::default() });
    }

    let peripherals = esp_hal::init(config);

    #[cfg(feature = "psram")]
    {
        log::info!("Using PSRAM");
        esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    }
    esp_alloc::heap_allocator!(
        #[link_section = ".dram2_uninit"]
        size: 64 * 1024
    );
    esp_alloc::heap_allocator!(size: 24 * 1024);

    let timer_group0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    assert!(RNG.set(rng).is_ok());

    #[cfg(not(feature = "esp32"))]
    {
        let systimer = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
        esp_hal_embassy::init(systimer.alarm0);
    }
    #[cfg(feature = "esp32")]
    {
        esp_hal_embassy::init(timer_group0.timer1);
    }

    let mut cpu_control = esp_hal::system::CpuControl::new(peripherals.CPU_CTRL);

    #[cfg(feature = "epaper_2_13")]
    let display_wrapper = {
        let spi = esp_hal::spi::master::Spi::new(
            peripherals.SPI2,
            esp_hal::spi::master::Config::default()
                .with_frequency(esp_hal::time::Rate::from_khz(100))
                .with_mode(esp_hal::spi::Mode::_0),
        )
        .unwrap()
        .with_sck(peripherals.GPIO18)
        .with_mosi(peripherals.GPIO23);

        let display_pin = esp_hal::gpio::Output::new(peripherals.GPIO5, esp_hal::gpio::Level::Low, esp_hal::gpio::OutputConfig::default());

        let mut spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, display_pin).unwrap();
        let busy_pin = esp_hal::gpio::Input::new(
            peripherals.GPIO4,
            esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::None),
        );
        let dc_pin = esp_hal::gpio::Output::new(peripherals.GPIO17, esp_hal::gpio::Level::High, esp_hal::gpio::OutputConfig::default());
        let rst_pin = esp_hal::gpio::Output::new(peripherals.GPIO16, esp_hal::gpio::Level::High, esp_hal::gpio::OutputConfig::default());
        let mut epaper_delay = esp_hal::delay::Delay::new();

        let epd = epd_waveshare::epd2in13_lillygo::Epd2in13::new(&mut spi_dev, busy_pin, dc_pin, rst_pin, &mut epaper_delay, None).unwrap();

        let display = epd_waveshare::epd2in13_lillygo::Display2in13::default();

        static DISPLAY_WRAPPER_RAW: static_cell::StaticCell<
            epaper_display_impl::LillyGoEPaper<
                embedded_hal_bus::spi::ExclusiveDevice<
                    esp_hal::spi::master::Spi<'_, esp_hal::Blocking>,
                    esp_hal::gpio::Output,
                    embedded_hal_bus::spi::NoDelay,
                >,
                esp_hal::gpio::Input,
                esp_hal::gpio::Output,
                esp_hal::gpio::Output,
                esp_hal::delay::Delay,
            >,
        > = static_cell::StaticCell::new();
        let display_wrapper = DISPLAY_WRAPPER_RAW.init_with(|| epaper_display_impl::LillyGoEPaper {
            spi_dev: spi_dev,
            epd: epd,
            display: display,
            delay: epaper_delay,
        });

        display_wrapper
    };

    #[cfg(feature = "scd30")]
    let sensor_wrapper = {
        let i2c = esp_hal::i2c::master::I2c::new(
            peripherals.I2C0,
            esp_hal::i2c::master::Config::default()
                .with_frequency(esp_hal::time::Rate::from_khz(50))
                .with_timeout(esp_hal::i2c::master::BusTimeout::Maximum),
        )
        .unwrap()
        .with_sda(peripherals.GPIO33)
        .with_scl(peripherals.GPIO32);

        let mut i2c_delay = esp_hal::delay::Delay::new();
        i2c_delay.delay_ns(5_000_000u32);

        let scd = sensor_scd30::Scd30::new(i2c, i2c_delay).unwrap();

        static SENSOR_WRAPPER_RAW: static_cell::StaticCell<
            scd30_sensor_impl::SCD30SensorWrapper<
                esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
                esp_hal::delay::Delay,
                esp_hal::i2c::master::Error,
            >,
        > = static_cell::StaticCell::new();

        let sensor_wrapper = SENSOR_WRAPPER_RAW.init_with(|| scd30_sensor_impl::SCD30SensorWrapper { sensor: scd });
        sensor_wrapper
    };

    cfg_if::cfg_if! {
        if #[cfg(feature = "psram")] {
            let sensor_channel = alloc::boxed::Box::leak(alloc::boxed::Box::new(
                embassy_sync::channel::Channel::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    edgeless_embedded::resource::scd30_sensor::Measurement,
                    2,
                >::new(),
            ));
        } else {
            static SENSOR_CHANNEL_RAW: static_cell::StaticCell<
                embassy_sync::channel::Channel<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    edgeless_embedded::resource::scd30_sensor::Measurement,
                    2,
                >,
            > = static_cell::StaticCell::new();


            let sensor_channel = SENSOR_CHANNEL_RAW.init_with(|| {
                embassy_sync::channel::Channel::<
                    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                    edgeless_embedded::resource::scd30_sensor::Measurement,
                    2,
                >::new()
            });
        }
    }

    #[allow(unused_variables)]
    let sender = sensor_channel.sender();
    #[allow(unused_variables)]
    let receiver = sensor_channel.receiver();

    cfg_if::cfg_if! {
        if #[cfg(feature = "psram")] {
            static DISPLAY_CHANNEL_RAW: static_cell::StaticCell<
                embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
            > = static_cell::StaticCell::new();
            let display_channel = DISPLAY_CHANNEL_RAW
                .init_with(|| embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>::new());
        } else {
            let display_channel = alloc::boxed::Box::leak(alloc::boxed::Box::new(embassy_sync::channel::Channel::<
                embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
                heapless::String<1500>,
                2,
            >::new()));
        }
    }

    #[allow(unused_variables)]
    let display_sender = display_channel.sender();
    #[allow(unused_variables)]
    let display_receiver = display_channel.receiver();

    let _other_core = cpu_control
        .start_app_core(
            unsafe {
                #[allow(static_mut_refs)]
                &mut APP_CORE_STACK
            },
            move || {
                static IO_EXECUTOR_RAW: static_cell::StaticCell<esp_hal_embassy::Executor> = static_cell::StaticCell::new();
                let io_executor = IO_EXECUTOR_RAW.init_with(esp_hal_embassy::Executor::new);

                io_executor.run(|#[allow(unused_variables)] spawner| {
                    #[cfg(feature = "epaper_2_13")]
                    display_wrapper.set_text("Edgeless");
                    #[cfg(feature = "scd30")]
                    spawner.spawn(io_task(spawner, sender, sensor_wrapper)).unwrap();
                    #[cfg(feature = "epaper_2_13")]
                    spawner
                        .spawn(edgeless_embedded::resource::epaper_display::display_writer(
                            display_receiver,
                            display_wrapper,
                        ))
                        .unwrap();
                });
            },
        )
        .unwrap();

    static EXECUTOR_RAW: static_cell::StaticCell<esp_hal_embassy::Executor> = static_cell::StaticCell::new();
    let executor = EXECUTOR_RAW.init_with(esp_hal_embassy::Executor::new);

    executor.run(|spawner| {
        spawner
            .spawn(edgeless(
                spawner,
                timer_group0.timer0,
                rng,
                peripherals.RADIO_CLK,
                peripherals.WIFI,
                receiver,
                display_sender,
            ))
            .unwrap();
    });
}

#[embassy_executor::task]
async fn io_task(
    spawner: embassy_executor::Spawner,
    sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        edgeless_embedded::resource::scd30_sensor::Measurement,
        2,
    >,
    sensor_wrapper: &'static mut dyn edgeless_embedded::resource::scd30_sensor::Sensor,
) {
    spawner
        .spawn(edgeless_embedded::resource::scd30_sensor::scd30_reader_task(sensor_wrapper, sender))
        .unwrap();
}

#[embassy_executor::task]
async fn edgeless(
    spawner: embassy_executor::Spawner,
    timer: esp_hal::timer::timg::Timer,
    rng: esp_hal::rng::Rng,
    radio_clock_control: esp_hal::peripherals::RADIO_CLK,
    wifi: esp_hal::peripherals::WIFI,
    #[allow(unused_variables)] sensor_scd_receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        edgeless_embedded::resource::scd30_sensor::Measurement,
        2,
    >,
    #[allow(unused_variables)] display_sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        heapless::String<1500>,
        2,
    >,
) {
    log::info!("Edgeless Embedded Async Main");

    cfg_if::cfg_if! {
        if #[cfg(feature = "psram")] {
            let rx_buf = alloc::boxed::Box::leak(alloc::boxed::Box::new([0 as u8; 2500]));
            let rx_meta = alloc::boxed::Box::leak(alloc::boxed::Box::new([embassy_net::udp::PacketMetadata::EMPTY; 10]));
            let tx_buf = alloc::boxed::Box::leak(alloc::boxed::Box::new([0 as u8; 2500]));
            let tx_meta = alloc::boxed::Box::leak(alloc::boxed::Box::new([embassy_net::udp::PacketMetadata::EMPTY; 10]));
            let app_tx = alloc::boxed::Box::leak(alloc::boxed::Box::new([0 as u8; 2500]));
            let app_rx = alloc::boxed::Box::leak(alloc::boxed::Box::new([0 as u8; 2500]));
        } else {
            static RX_BUF_RAW: static_cell::StaticCell<[u8; 2500]> = static_cell::StaticCell::new();
            let rx_buf = RX_BUF_RAW.init_with(|| [0_u8; 2500]);
            static RX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
            let rx_meta = RX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);
            static TX_BUF_RAW: static_cell::StaticCell<[u8; 2500]> = static_cell::StaticCell::new();
            let tx_buf = TX_BUF_RAW.init_with(|| [0_u8; 2500]);
            static TX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
            let tx_meta = TX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);
            static APP_TX_RAW: static_cell::StaticCell<[u8; 2500]> = static_cell::StaticCell::new();
            let app_tx = APP_TX_RAW.init_with(|| [0_u8; 2500]);
            static APP_RX_RAW: static_cell::StaticCell<[u8; 2500]> = static_cell::StaticCell::new();
            let app_rx = APP_RX_RAW.init_with(|| [0_u8; 2500]);
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "scd30")] {
            let sensor_scd30_resource = edgeless_embedded::resource::scd30_sensor::SCD30Sensor::new_resource(sensor_scd_receiver).await;
        } else {
            let sensor_scd30_resource = edgeless_embedded::resource::mock_sensor::MockSensor::new_resource().await;
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "epaper_2_13")] {
            let display_resource = edgeless_embedded::resource::epaper_display::EPaperDisplay::new_resource(display_sender).await;
        } else {
            let display_resource = edgeless_embedded::resource::mock_display::MockDisplay::new_resource().await;
        }
    }

    static RESOURCES_RAW: static_cell::StaticCell<[&'static mut dyn edgeless_embedded::resource::ResourceDyn; 2]> = static_cell::StaticCell::new();
    let resources = RESOURCES_RAW.init_with(|| [sensor_scd30_resource, display_resource]);

    #[cfg(feature = "wasm")]
    let agent = {
        log::info!("Using a WASM Runtime");
        cfg_if::cfg_if! {
            if #[cfg(feature = "psram")] {
                let wasm_runtime = alloc::boxed::Box::leak(alloc::boxed::Box::new(edgeless_embedded::wasm_functions::WasmiRuntime::new()));
            } else {
                static WASM_RUNTIME_RAW: static_cell::StaticCell<edgeless_embedded::wasm_functions::WasmiRuntime> = static_cell::StaticCell::new();
                let wasm_runtime = WASM_RUNTIME_RAW.init_with(|| edgeless_embedded::wasm_functions::WasmiRuntime::new());
            }
        };

        edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID.clone(), Some(wasm_runtime), resources).await
    };
    #[cfg(not(feature = "wasm"))]
    let agent = edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID, None, resources).await;

    log::info!("Agent Created");

    let stack = wifi::init(spawner, timer, rng, radio_clock_control, wifi, agent.clone()).await;
    let sock = embassy_net::udp::UdpSocket::new(stack, rx_meta, rx_buf, tx_meta, tx_buf);

    log::info!("WiFi Started");

    spawner
        .spawn(edgeless_embedded::coap::coap_task(
            sock,
            agent.upstream_receiver().unwrap(),
            agent.clone(),
            app_rx,
            app_tx,
        ))
        .unwrap();

    log::info!("CoAP Started");
}
