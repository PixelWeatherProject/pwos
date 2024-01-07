use crate::{
    config::{AppConfig, MAX_NET_SCAN, PWMP_SERVER, WIFI_NETWORKS},
    os_debug, os_error, os_info, os_warn,
    sysc::{
        battery::{Battery, CRITICAL_VOLTAGE},
        drivers::{AnySensor, EnvironmentSensor, Htu21d, MeasurementResults, Si7021},
        ledctl::BoardLed,
        net::{PowerSavingMode, WiFi},
        sleep::deep_sleep,
        OsError, OsResult,
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
use pwmp_client::{pwmp_types::setting::SettingName, PwmpClient};
#[cfg(debug_assertions)]
use std::time::Instant;

pub fn fw_main(
    peripherals: Peripherals,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    mut led: BoardLed,
    cfg: &mut AppConfig,
) -> OsResult<()> {
    let i2c_driver = I2cDriver::new(
        peripherals.i2c1,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &Config::default().baudrate(400u32.kHz().into()),
    )?;
    let mut htu = Htu21d::new_with_driver(i2c_driver)
        .map_err(|err| err.0)
        .unwrap();

    let serial = htu.read_serial();
    os_info!("{serial:?}");

    Ok(())
}

fn setup_wifi(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> OsResult<(WiFi<'static>, AccessPointInfo)> {
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
        os_info!("Connecting to {}", ap.ssid);

        let psk = WIFI_NETWORKS
            .iter()
            .find(|entry| entry.0 == ap.ssid)
            .unwrap()
            .1;

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

    let values = pws.get_settings([
        SettingName::BatteryIgnore,
        SettingName::Ota,
        SettingName::SleepTime,
        SettingName::Sbop,
        SettingName::MuteNotifications,
    ])?;

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
        Some(Si7021::DEV_ADDR) => {
            /* The SI7021 and HTU21D have the same address, so we'll try both drivers. */

            match Si7021::new_with_driver(i2c_driver) {
                Ok(si) => {
                    os_debug!("Detected SI7021");
                    Ok(AnySensor::Si7021(si))
                }
                Err((_, driver)) => Htu21d::new_with_driver(driver).map_or_else(
                    |_| Err(OsError::NoEnvSensor),
                    |htu| {
                        os_debug!("Detected HTU21D");
                        Ok(AnySensor::Htu21d(htu))
                    },
                ),
            }
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
