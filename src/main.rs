#![allow(
    clippy::module_name_repetitions,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::config::AppConfig;
use build_time::build_time_local;
use config::{ENABLE_SHELL, LED_BUILTIN};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        i2c::{config::Config, I2cDriver},
        peripherals::Peripherals,
        units::FromValueType,
    },
    nvs::EspDefaultNvsPartition,
};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::time::Instant;
use sysc::{
    battery::Battery,
    gpio,
    ledctl::BoardLed,
    ota::Ota,
    sleep::{deep_sleep, fake_sleep},
    usbctl, OsError,
};

mod config;
mod firmware;
mod shell;
mod sysc;

/// Storage for a recoverable error that may have occurred during a previous run.
///
/// ## Note
/// This variable is not preserved when the node is connected to a PC for an unknown reason.
#[link_section = ".rtc.data"]
static mut LAST_ERROR: Option<OsError> = Option::None;

fn main() {
    esp_idf_svc::sys::link_patches();

    let mut logger = SimpleLogger::new().with_module_level("esp_idf_svc", LevelFilter::Off);

    if cfg!(debug_assertions) {
        logger = logger.with_level(LevelFilter::Debug);
    }

    // Turn off logging when USB is not connected
    if !usbctl::is_connected() {
        logger = logger.with_level(LevelFilter::Off);
    }

    logger.init().unwrap();

    os_info!(
        "PixelWeatherOS v{}-{}{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("PWOS_COMMIT"),
        env!("PWOS_REL_OR_DEV"),
        build_time_local!("%d.%m.%Y %H:%M:%S")
    );
    os_info!("(C) Fábián Varga 2025");

    #[cfg(debug_assertions)]
    {
        let raw_version = unsafe { esp_idf_svc::sys::esp_get_idf_version() };
        let version = unsafe { std::ffi::CStr::from_ptr(raw_version.cast()) };
        os_debug!("Using ESP-IDF {}", version.to_string_lossy());
    }

    os_debug!("Disabling brownout detector");
    sysc::brownout::disable_brownout_detector();

    os_debug!("Initializing peripherals");
    let mut peripherals = Peripherals::take().expect("Failed to initialize peripherals");

    os_debug!("Initializing System Event Loop");
    let sys_loop = EspSystemEventLoop::take().expect("SEL init error");

    os_debug!("Initializing NVS Storage");
    let nvs = EspDefaultNvsPartition::take().expect("NVS init error");

    os_debug!("Initializing system LED");
    let led = BoardLed::new(
        gpio::number_to_io_pin(LED_BUILTIN, &mut peripherals).expect("Invalid LED pin"),
    );

    os_debug!("Initializing OTA system");
    let mut ota = Ota::new().expect("Failed to initialize OTA");

    ota.rollback_if_needed()
        .expect("Failed to check/perform rollback");

    os_debug!("Initializing system Battery");
    let battery = Battery::new(peripherals.adc1, peripherals.pins.gpio2)
        .expect("Failed to initialize battery ADC");

    os_debug!("Initializing I2C bus");
    let i2c = I2cDriver::new(
        peripherals.i2c1,
        peripherals.pins.gpio5,
        peripherals.pins.gpio8,
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
        peripherals.modem,
        sys_loop,
        nvs,
        led,
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

            // SAFETY: Since this program is not multithreaded, this will always be safe.
            #[allow(static_mut_refs)]
            unsafe {
                LAST_ERROR.replace(why)
            };

            if ENABLE_SHELL {
                shell::shell_start();
            }

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
