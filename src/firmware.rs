use crate::{
    config::{AppConfig, MAX_NET_SCAN, PWMP_SERVER, WIFI_NETWORKS, WIFI_TIMEOUT},
    os_debug, os_error, os_info, os_warn,
    sysc::{
        battery::{Battery, CRITICAL_VOLTAGE},
        ext_drivers::{AnySensor, EnvironmentSensor, Htu, MeasurementResults},
        ledctl::BoardLed,
        net::{PowerSavingMode, WiFi},
        ota::Ota,
        sleep::deep_sleep,
        usbctl, OsError, OsResult, ReportableError,
    },
    LAST_ERROR,
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{i2c::I2cDriver, modem::Modem},
    nvs::EspDefaultNvsPartition,
    wifi::AccessPointInfo,
};
use pwmp_client::{
    ota::UpdateStatus,
    pwmp_msg::{dec, version::Version, Decimal},
    PwmpClient,
};
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn fw_main(
    mut battery: Battery,
    i2c: I2cDriver,
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    mut led: BoardLed,
    ota: &mut Ota,
    cfg: &mut AppConfig,
) -> OsResult<()> {
    if !ota.current_verified()? {
        os_warn!("Running unverified firmware");
    }

    let (wifi, ap) = setup_wifi(modem, sys_loop, nvs)?;
    let mut pws = PwmpClient::new(PWMP_SERVER, wifi.get_mac()?, None, None, None)?;

    read_appcfg(&mut pws, cfg)?;

    let bat_voltage = if usbctl::is_connected() {
        os_debug!("Skipping battery voltage measurement due to USB power");
        dec!(5.00)
    } else {
        battery.read(16)?
    };
    os_info!("Battery: {bat_voltage}V");

    if (bat_voltage <= CRITICAL_VOLTAGE) && cfg.sbop {
        os_warn!("Battery voltage too low, activating sBOP");

        pws.send_notification("Battery voltage too low, activating sBOP")
            .report("Failed to send sBOP notification");

        deep_sleep(None);
    }

    let env_sensor = setup_envsensor(i2c)?;

    let results = read_environment(env_sensor)?;
    os_info!("{}*C / {}%", results.temperature, results.humidity);
    os_debug!("Posting measurements");
    pws.post_measurements(results.temperature, results.humidity, results.air_pressure)?;

    os_debug!("Posting stats");
    pws.post_stats(bat_voltage, &ap.ssid, ap.signal_strength)?;

    // SAFETY: Since this program is not multithreaded, this will always be safe.
    #[allow(static_mut_refs)]
    if let Some(error) = unsafe {
        LAST_ERROR.take() /* also clears the Option */
    } {
        os_info!("Reporting error from previous run");

        pws.send_notification(format!(
            "An error has been detected during a previous run: {error}"
        ))
        .report("Failed to report previous error");
    } else {
        os_debug!("No error detected from previous run")
    }

    if ota.report_needed()? {
        let success = !ota.rollback_detected()?;

        os_info!(
            "Reporting {}successfull firmware update",
            if success { "" } else { "un" }
        );

        pws.send_notification(format!(
            "Update to PWOS {} has {}",
            if success {
                env!("CARGO_PKG_VERSION").to_string()
            } else {
                ota.previous_version()?.unwrap().to_string()
            },
            if success { "succeeded" } else { "failed" }
        ))?;

        pws.report_firmware(success)?;
        ota.mark_reported();
    } else {
        os_debug!("No update report needed");
    }

    os_debug!("Checking for updates");
    if check_ota(&mut pws)? {
        begin_update(&mut pws, ota)?;
    }

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
    let scan_start = std::time::Instant::now();
    let mut networks = wifi.scan::<MAX_NET_SCAN>(Duration::from_secs(2))?;

    #[cfg(debug_assertions)]
    {
        let network_names = networks
            .iter()
            .map(|net| net.ssid.as_str())
            .collect::<heapless::Vec<&str, MAX_NET_SCAN>>();

        os_debug!(
            "Found networks: {network_names:?} in {:.02?}",
            scan_start.elapsed()
        );
    }

    // filter out unknown APs
    networks.retain(|ap| WIFI_NETWORKS.iter().any(|entry| entry.0 == ap.ssid));
    // sort by signal strength
    networks.sort_by(|a, b| b.signal_strength.partial_cmp(&a.signal_strength).unwrap());
    // filter out APs with RSSI >= -90
    networks.retain(|ap| ap.signal_strength >= -90);

    if networks.is_empty() {
        os_warn!("No usable networks found");
        return Err(OsError::NoInternet);
    }

    for ap in networks {
        os_debug!("Connecting to {} ({}dBm)", ap.ssid, ap.signal_strength);

        // SAFETY: Unknown APs are filtered out, so `find`` will always return something.
        let psk = unsafe {
            WIFI_NETWORKS
                .iter()
                .find(|entry| entry.0 == ap.ssid)
                .unwrap_unchecked()
                .1
        };

        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();
        match wifi.connect(&ap.ssid, psk, ap.auth_method.unwrap(), WIFI_TIMEOUT) {
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

    if let Some(settings) = pws.get_settings()? {
        **appcfg = settings;
    } else {
        os_warn!("Got empty node settings, using defaults");
    }

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
            return Ok(AnySensor::HtuCompatible(Htu::new_with_driver(i2c_driver)?));
        }
        Some(other) => {
            os_warn!("Unrecognised device @ I2C/0x{other:X}");
        }
        None => (),
    }

    Err(OsError::NoEnvSensor)
}

fn read_environment(mut env: AnySensor) -> OsResult<MeasurementResults> {
    Ok(MeasurementResults {
        temperature: env.read_temperature()?,
        humidity: env.read_humidity()?,
        air_pressure: env.read_air_pressure()?,
    })
}

fn check_ota(pws: &mut PwmpClient) -> OsResult<bool> {
    let current_version =
        Version::parse(env!("CARGO_PKG_VERSION")).ok_or(OsError::IllegalFirmwareVersion)?;

    match pws.check_os_update(current_version)? {
        UpdateStatus::UpToDate => {
            os_info!("No update available");
            Ok(false)
        }
        UpdateStatus::Available(new_version) => {
            os_info!("Update v{new_version} available");
            Ok(true)
        }
    }
}

fn begin_update(pws: &mut PwmpClient, ota: &mut Ota) -> OsResult<()> {
    let mut handle = ota.begin_update()?;
    let mut maybe_chunk = pws.next_update_chunk(Some(1024))?;
    let mut i = 1;

    while let Some(chunk) = maybe_chunk {
        if i % 128 == 0 {
            os_debug!("Writing OTA update chunk #{i}");
        }

        handle.write(&chunk)?;
        maybe_chunk = pws.next_update_chunk(Some(1024))?;
        i += 1;
    }

    drop(handle); // This will internally finalize the update
    os_info!("Update installed successfully");

    Ok(())
}
