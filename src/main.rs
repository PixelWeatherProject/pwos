#![warn(clippy::unwrap_used)]
#![deny(unused_must_use)]

use crate::sysc::ReportableError;
use esp_idf_svc::hal::{
    gpio::IOPin,
    i2c::{config::Config, I2cDriver},
    units::FromValueType,
};
use std::time::Instant;
use sysc::{
    battery::Battery, ledctl::BoardLed, logging::OsLogger, ota::Ota, periph::SystemPeripherals,
    power::mcu_sleep, usbctl, OsError,
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

    log::info!(
        "PixelWeatherOS v{}-{}{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("PWOS_COMMIT"),
        env!("PWOS_REL_OR_DEV"),
        env!("BUILD_DATE_TIME")
    );
    log::info!("(C) Fábián Varga 2025");

    #[cfg(debug_assertions)]
    {
        log::debug!(
            "Using ESP-IDF {}",
            sysc::get_idf_version().as_deref().unwrap_or("?")
        );
        log::debug!("Disabling brownout detector");
        sysc::brownout::disable_brownout_detector();
    }

    log::debug!("Initializing system peripherals");
    let peripherals = SystemPeripherals::take();

    log::debug!("Initializing system LED");
    let led = BoardLed::new(
        peripherals.onboard_led.pin.downgrade(),
        peripherals.onboard_led.invert,
    )
    .expect("Failed to set up onboard LED");

    log::debug!("Setting panic handle");
    sysc::panic::setup();

    log::debug!("Initializing OTA system");
    let mut ota = Ota::new().expect("Failed to initialize OTA");

    ota.rollback_if_needed()
        .expect("Failed to check/perform rollback");

    log::debug!(
        "Reported current version: {}",
        ota.current_version()
            .map_or_else(|_| "?".to_string(), |v| v.to_string())
    );
    log::debug!(
        "Previous installed version: {}",
        ota.previous_version()
            .map_or_else(|_| "?".to_string(), |v| v.to_string())
    );

    log::debug!("Initializing NVS");
    let mut nvs = sysc::nvs::NonVolatileStorage::new().expect("Failed to initialize NVS");

    log::debug!("Initializing system Battery");
    let battery = Battery::new(peripherals.battery.adc, peripherals.battery.pin)
        .expect("Failed to initialize battery ADC");

    log::debug!("Initializing I2C bus");
    let i2c = I2cDriver::new(
        peripherals.i2c.i2c,
        peripherals.i2c.sda,
        peripherals.i2c.scl,
        &Config::default().baudrate(400u32.kHz().into()),
    )
    .expect("Failed to initialize I2C");

    log::debug!("Initializing app configuration");
    let mut appcfg = config::get_settings();

    log::info!("Staring main");

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
        Ok(()) => log::info!("Tasks completed successfully"),
        Err(why) => {
            log::error!("OS Error: {why}");

            nvs.store_last_os_error(&why)
                .report("Failed to store error in NVS");

            if !why.recoverable() {
                log::error!("System will now halt");
                mcu_sleep(None);
            }

            ota.inc_failiures()
                .expect("Failed to increment failiure count");
        }
    }
    log::info!("Tasks completed in {runtime:.02?}");

    log::debug!("Sleeping for {:?}", appcfg.sleep_time());
    mcu_sleep(Some(appcfg.sleep_time()));
}
