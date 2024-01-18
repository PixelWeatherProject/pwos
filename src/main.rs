#![allow(
    clippy::module_name_repetitions,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::config::AppConfig;
use build_time::build_time_local;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{peripheral::Peripheral, peripherals::Peripherals},
    nvs::EspDefaultNvsPartition,
};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::time::Instant;
use sysc::{ledctl::BoardLed, sleep::deep_sleep};

mod config;
mod firmware;
mod sysc;

fn main() {
    esp_idf_svc::sys::link_patches();

    let logger = SimpleLogger::new().with_module_level("esp_idf_svc", LevelFilter::Off);

    #[cfg(not(debug_assertions))]
    let logger = logger.with_level(LevelFilter::Info);

    logger.init().unwrap();

    os_info!(
        "PixelWeatherOS v{} ({})",
        env!("CARGO_PKG_VERSION"),
        build_time_local!("%d.%m.%Y %H:%M:%S")
    );
    os_info!("(C) Fábián Varga 2024");

    os_debug!("Disabling brownout detector");
    sysc::brownout::disable_brownout_detector();

    os_debug!("Initializing peripherals");
    let mut peripherals = Peripherals::take().expect("Failed to initialize peripherals");

    os_debug!("Initializing System Event Loop");
    let sys_loop = EspSystemEventLoop::take().expect("SEL init error");

    os_debug!("Initializing NVS Storage");
    let nvs = EspDefaultNvsPartition::take().expect("NVS init error");

    os_debug!("Initializing system LED");
    let led = BoardLed::new(unsafe { peripherals.pins.gpio19.clone_unchecked() });

    os_debug!("Initializing app configuration");
    let mut appcfg = AppConfig::default();

    os_info!("Staring main");

    let start = Instant::now();
    let fw_exit = firmware::fw_main(peripherals, sys_loop, nvs, led, &mut appcfg);
    let runtime = start.elapsed();

    match fw_exit {
        Ok(()) => os_info!("Tasks completed successfully"),
        Err(why) => {
            if !why.recoverable() {
                os_error!("Fatal OS Error: {why}");
                deep_sleep(None);
            }
            os_error!("OS Error: {why}");
        }
    }
    os_info!("Tasks completed in {runtime:.02?}s");

    os_debug!("Sleeping for {}s", appcfg.sleep_time.as_secs());
    deep_sleep(Some(appcfg.sleep_time));
}
