#![warn(clippy::unwrap_used)]
#![feature(panic_payload_as_str)]

use crate::config::AppConfig;
use esp_idf_svc::hal::{
    gpio::IOPin,
    i2c::{config::Config, I2cDriver},
    units::FromValueType,
};
use std::{panic::PanicHookInfo, time::Instant};
use sysc::{
    battery::Battery,
    ledctl::BoardLed,
    logging::OsLogger,
    ota::Ota,
    periph::SystemPeripherals,
    power::{deep_sleep, fake_sleep},
    usbctl, OsError,
};

mod config;
mod firmware;
mod sysc;

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn main() {
    esp_idf_svc::sys::link_patches();

    let mut logger = OsLogger::new();

    // Turn off logging when USB is not connected
    if !usbctl::is_connected() {
        logger.disable();
    }

    logger.init();

    os_info!(
        "PixelWeatherOS v{}-{}{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("PWOS_COMMIT"),
        env!("PWOS_REL_OR_DEV"),
        env!("BUILD_DATE_TIME")
    );
    os_info!("(C) Fábián Varga 2025");

    #[cfg(debug_assertions)]
    {
        os_debug!(
            "Using ESP-IDF {}",
            sysc::get_idf_version().as_deref().unwrap_or("?")
        );
        os_debug!("Disabling brownout detector");
        sysc::brownout::disable_brownout_detector();
    }

    os_debug!("Initializing system peripherals");
    let peripherals = SystemPeripherals::take();

    os_debug!("Initializing system LED");
    let led = BoardLed::new(
        peripherals.onboard_led.pin.downgrade(),
        peripherals.onboard_led.invert,
    )
    .expect("Failed to set up onboard LED");

    os_debug!("Setting panic handle");
    std::panic::set_hook(Box::new(handle_panic));

    os_debug!("Initializing OTA system");
    let mut ota = Ota::new().expect("Failed to initialize OTA");

    ota.rollback_if_needed()
        .expect("Failed to check/perform rollback");

    os_debug!(
        "Reported current version: {}",
        ota.current_version()
            .ok()
            .flatten()
            .map_or_else(|| "?".to_string(), |v| v.to_string())
    );
    os_debug!(
        "Previous installed version: {}",
        ota.previous_version()
            .ok()
            .flatten()
            .map_or_else(|| "?".to_string(), |v| v.to_string())
    );

    os_debug!("Initializing NVS");
    let mut nvs = sysc::nvs::NonVolatileStorage::new().expect("Failed to initialize NVS");

    os_debug!("Initializing system Battery");
    let battery = Battery::new(peripherals.battery.adc, peripherals.battery.pin)
        .expect("Failed to initialize battery ADC");

    os_debug!("Initializing I2C bus");
    let i2c = I2cDriver::new(
        peripherals.i2c.i2c,
        peripherals.i2c.sda,
        peripherals.i2c.scl,
        &Config::default().baudrate(400u32.kHz().into()),
    )
    .expect("Failed to initialize I2C");

    os_debug!("Initializing app configuration");
    let mut appcfg = AppConfig::default();

    os_info!("Staring main");

    let start = Instant::now();
    let fw_exit = firmware::fw_main(
        battery,
        i2c,
        peripherals.wifi.modem,
        peripherals.wifi.sys_loop,
        led,
        &mut nvs,
        &mut ota,
        &mut appcfg,
    );
    let runtime = start.elapsed();

    match fw_exit {
        Ok(()) => os_info!("Tasks completed successfully"),
        Err(why) => {
            os_error!("OS Error: {why}");

            if !why.recoverable() {
                os_error!("System will now halt");
                deep_sleep(None);
            }

            if let Err(why) = nvs.store_last_os_error(&why) {
                os_error!("Failed to store error in NVS: {why}");
            };

            ota.inc_failiures()
                .expect("Failed to increment failiure count");
        }
    }
    os_info!("Tasks completed in {runtime:.02?}");

    os_debug!("Sleeping for {:?}", appcfg.sleep_time());

    if usbctl::is_connected() {
        // Simulate sleep instead, to keep the serial connection alive
        os_debug!("Using fake sleep instead of deep sleep");
        fake_sleep(Some(appcfg.sleep_time()));
    } else {
        deep_sleep(Some(appcfg.sleep_time()));
    }
}

fn handle_panic(info: &PanicHookInfo) {
    let payload = info.payload_as_str().unwrap_or("N/A");

    os_error!("====================[PANIC]====================");
    os_error!("Firmware paniced!");
    os_error!("Message: {payload}");
    os_error!(
        "Location: {}",
        info.location()
            .map_or_else(|| "N/A".to_string(), ToString::to_string)
    );
    os_error!("====================[PANIC]====================");
}
