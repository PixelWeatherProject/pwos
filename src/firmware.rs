use crate::{
    config::{AppConfig, MAX_NET_SCAN, PWMP_SERVER, WIFI_NETWORKS},
    os_debug, os_error, os_info, os_warn,
    sysc::{
        battery::{Battery, CRITICAL_VOLTAGE},
        drivers::{AnySensor, EnvironmentSensor, Htu, MeasurementResults},
        ledctl::BoardLed,
        net::{PowerSavingMode, WiFi},
        sleep::deep_sleep,
        OsError, OsResult, ReportableError,
    },
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        i2c::{config::Config, I2cDriver},
        modem::Modem,
        peripherals::Peripherals,
        units::FromValueType,
    },
    nvs::EspDefaultNvsPartition,
    wifi::AccessPointInfo,
};
use pwmp_client::PwmpClient;
#[cfg(debug_assertions)]
use std::time::Instant;

pub fn fw_main(
    peripherals: Peripherals,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    mut led: BoardLed,
    cfg: &mut AppConfig,
) -> OsResult<()> {
    let (wifi, ap) = setup_wifi(peripherals.modem, sys_loop, nvs)?;
    let mut pws = PwmpClient::new(PWMP_SERVER, wifi.get_mac()?)?;

    read_appcfg(&mut pws, cfg)?;

    let mut battery = Battery::new(peripherals.adc1, peripherals.pins.gpio35)?;
    let bat_voltage = battery.read_voltage(4)?;
    os_info!("Battery: {bat_voltage}V");

    if (bat_voltage <= CRITICAL_VOLTAGE) && cfg.sbop {
        os_warn!("Battery voltage too low, activating sBOP");

        pws.send_notification("Battery voltage too low, activating sBOP")
            .report("Failed to send sBOP notification");

        deep_sleep(None);
    }

    let i2c_driver = I2cDriver::new(
        peripherals.i2c1,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &Config::default().baudrate(400u32.kHz().into()),
    )?;
    let env_sensor = setup_envsensor(i2c_driver)?;

    let results = read_environment(env_sensor)?;
    os_info!("{}*C / {}%", results.temperature, results.humidity);
    os_debug!("Posting measurements");
    pws.post_measurements(results.temperature, results.humidity, results.air_pressure)?;

    os_debug!("Posting stats");
    pws.post_stats(bat_voltage, &ap.ssid, ap.signal_strength)?;

    // Peacefully disconnect
    drop(pws);

    led.off();
    Ok(())
}

fn setup_wifi(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> OsResult<(WiFi, AccessPointInfo)> {
    os_debug!("Initializing WiFi");
    let mut wifi = WiFi::new(modem, sys_loop, nvs)?;

    wifi.set_power_saving(PowerSavingMode::Minimum)?;
    wifi.set_power(84)?;

    #[cfg(debug_assertions)]
    let scan_start = Instant::now();
    let mut networks = wifi.scan::<MAX_NET_SCAN>()?;
    #[cfg(debug_assertions)]
    {
        let network_names = networks
            .iter()
            .map(|net| net.ssid.as_str())
            .collect::<heapless::Vec<&str, MAX_NET_SCAN>>();

        os_debug!(
            "Found networks: {network_names:?} in {:?}",
            scan_start.elapsed()
        );
    }

    // filter out unknown APs
    networks.retain(|ap| WIFI_NETWORKS.iter().any(|entry| entry.0 == ap.ssid));
    // sort by signal strength
    networks.sort_by(|a, b| b.signal_strength.partial_cmp(&a.signal_strength).unwrap());

    for ap in networks {
        os_debug!("Connecting to {}", ap.ssid);

        // SAFETY: Unknown APs are filtered out, so `find`` will always return something.
        let psk = unsafe {
            WIFI_NETWORKS
                .iter()
                .find(|entry| entry.0 == ap.ssid)
                .unwrap_unchecked()
                .1
        };

        #[cfg(debug_assertions)]
        let start = Instant::now();
        match wifi.connect(&ap.ssid, psk, ap.auth_method) {
            Ok(()) => {
                os_debug!("Connected in {:?}", start.elapsed());
                os_debug!("IP: {}", wifi.get_ip_info().unwrap().ip);
                return Ok((wifi, ap));
            }
            Err(why) => os_error!("Failed to connect: {why}"),
        }
    }

    Err(OsError::NoInternet)
}

fn read_appcfg(pws: &mut PwmpClient, appcfg: &mut AppConfig) -> OsResult<()> {
    os_debug!("Reading settings");

    let values = pws.get_settings(AppConfig::ALL_SETTINGS)?;
    appcfg.update_settings(values);

    os_debug!("Settings updated");
    Ok(())
}

fn setup_envsensor(mut i2c_driver: I2cDriver<'_>) -> OsResult<AnySensor<'_>> {
    let mut working = None;

    for addr in 1..128 {
        if i2c_driver.write(addr, &[], 1000).is_ok() {
            os_debug!("Found device @ I2C/0x{addr:X}");
            working = Some(addr);

            /* We expect only ONE device, so the loop can be broken here. */
            break;
        }
    }

    match working {
        Some(Htu::DEV_ADDR) => {
            os_debug!("Detected HTU-compatible sensor");
            Ok(AnySensor::HtuCompatible(Htu::new_with_driver(i2c_driver)?))
        }
        Some(other) => {
            os_error!("Unrecognised device @ I2C/0x{other:X}");
            Err(OsError::NoEnvSensor)
        }
        None => Err(OsError::NoEnvSensor),
    }
}

fn read_environment(mut env: AnySensor) -> OsResult<MeasurementResults> {
    Ok(MeasurementResults {
        temperature: env.read_temperature()?,
        humidity: env.read_humidity()?,
        air_pressure: env.read_air_pressure()?,
    })
}
