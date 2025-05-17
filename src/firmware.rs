use crate::{
    config::{AppConfig, PWMP_SERVER, WIFI_NETWORKS, WIFI_TIMEOUT},
    null_check, os_debug, os_error, os_info, os_warn, re_esp,
    sysc::{
        battery::{Battery, CRITICAL_VOLTAGE},
        ext_drivers::{AnySensor, EnvironmentSensor, Htu, MeasurementResults},
        ledctl::BoardLed,
        net::wifi::{WiFi, RSSI_THRESHOLD},
        nvs::NonVolatileStorage,
        ota::{Ota, OtaHandle},
        power::{deep_sleep, get_reset_reason, ResetReasonExt},
        usbctl, OsError, OsResult, ReportableError,
    },
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{i2c::I2cDriver, modem::Modem},
    sys::esp_random,
    wifi::AccessPointInfo,
};
use pwmp_client::{
    ota::UpdateStatus,
    pwmp_msg::{version::Version, MsgId},
    PwmpClient,
};

#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
pub fn fw_main(
    mut battery: Battery,
    i2c: I2cDriver,
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    mut led: BoardLed,
    nvs: &mut NonVolatileStorage,
    ota: &mut Ota,
    cfg: &mut AppConfig,
) -> OsResult<()> {
    if !ota.current_verified()? {
        os_warn!("Running unverified firmware");
    }

    os_debug!("Starting WiFi setup");
    let (wifi, ap) = setup_wifi(modem, sys_loop)?;
    os_debug!("Connecting to PWMP");
    let mut pws = PwmpClient::new(PWMP_SERVER, &pwmp_msg_id_rng, None, None, None)?;

    os_debug!("Sending handshake request");
    pws.perform_handshake(wifi.get_mac()?)?;

    os_debug!("Requesting app configuration");
    read_appcfg(&mut pws, cfg)?;

    let bat_voltage = battery.read(64)?;
    if usbctl::is_connected() {
        os_warn!("Battery voltage measurement may be affected by USB power");
        cfg.battery_ignore = true;
    }
    os_info!("Battery: {bat_voltage:.02}V");

    if (bat_voltage <= CRITICAL_VOLTAGE) && cfg.sbop && !cfg.battery_ignore {
        os_warn!("Battery voltage too low, activating sBOP");

        pws.send_notification("Battery voltage too low, activating sBOP")
            .report("Failed to send sBOP notification");

        deep_sleep(None);
    }

    let env_sensor = setup_envsensor(i2c)?;

    let results = read_environment(env_sensor)?;
    os_info!("{:.02}*C / {}%", results.temperature, results.humidity);
    os_debug!("Posting measurements");
    pws.post_measurements(results.temperature, results.humidity, results.air_pressure)?;

    os_debug!("Posting stats");
    pws.post_stats(bat_voltage, &ap.ssid, ap.signal_strength)?;

    let reset_reason = get_reset_reason();
    if reset_reason.is_abnormal() {
        os_warn!("Detected abnormal reset reason: {reset_reason:?}");

        pws.send_notification(format!(
            "Detected abnormal reset reason: {:?}",
            get_reset_reason()
        ))
        .report("Failed to report abnormal reset reason");
    } else {
        os_debug!("Reset reason ({reset_reason:?}) is normal");
    }

    if let Some(error) = nvs.get_last_os_error()? {
        os_info!("Reporting error from previous run ({error})");

        pws.send_notification(format!(
            "An error has been detected during a previous run: {error}"
        ))
        .report("Failed to report previous error");

        nvs.clear_last_os_error()?;
    } else {
        os_debug!("No error detected from previous run");
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
                ota.previous_version()?
                    .map_or_else(|| "unknown".to_string(), |v| v.to_string())
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
        let mut handle = ota.begin_update()?;

        if let Err(why) = begin_update(&mut pws, &mut handle) {
            os_error!("OTA failed: {why}, aborting");
            handle.cancel()?;
        }

        os_info!("Update installed successfully");
    } // Handle will be dropped and the update should finalize

    led.off();
    Ok(())
}

fn setup_wifi(modem: Modem, sys_loop: EspSystemEventLoop) -> OsResult<(WiFi, AccessPointInfo)> {
    os_debug!("Initializing WiFi");
    let mut wifi = WiFi::new(modem, sys_loop)?;

    os_debug!("Starting WiFi scan");
    #[cfg(debug_assertions)]
    let scan_start = std::time::Instant::now();
    let mut networks = wifi.scan()?;

    #[cfg(debug_assertions)]
    {
        use crate::sysc::net::wifi::MAX_NET_SCAN;

        let network_names = networks
            .iter()
            .map(|net| net.ssid.as_str())
            .collect::<heapless::Vec<&str, { MAX_NET_SCAN }>>();

        os_debug!(
            "Found networks: {network_names:?} in {:.02?}",
            scan_start.elapsed()
        );
    }

    // filter out unknown APs
    networks.retain(|ap| WIFI_NETWORKS.iter().any(|entry| entry.0 == ap.ssid));
    // sort by signal strength
    networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
    // filter out APs with RSSI >= RSSI_THRESHOLD
    networks.retain(|ap| ap.signal_strength >= RSSI_THRESHOLD);

    if networks.is_empty() {
        os_warn!("No usable networks found");
        return Err(OsError::NoInternet);
    }

    for ap in networks {
        os_debug!("Connecting to {} ({}dBm)", ap.ssid, ap.signal_strength);
        let psk = null_check!(WIFI_NETWORKS
            .iter()
            .find(|entry| entry.0 == ap.ssid)
            .map(|e| e.1));

        #[cfg(debug_assertions)]
        let start = std::time::Instant::now();
        match wifi.connect(&ap, psk, WIFI_TIMEOUT) {
            Ok(()) => {
                os_debug!("Connected in {:.02?}", start.elapsed());
                os_debug!("IP: {}", wifi.get_ip_info()?.ip);
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

fn begin_update(pws: &mut PwmpClient, handle: &mut OtaHandle) -> OsResult<()> {
    let mut maybe_chunk = pws.next_update_chunk(Some(1024))?;
    let mut i = 1;

    while let Some(chunk) = maybe_chunk {
        if i % 128 == 0 {
            os_debug!("Writing OTA update chunk #{i}");
        }

        re_esp!(handle.write(&chunk), OtaWrite)?;
        maybe_chunk = pws.next_update_chunk(Some(1024))?;
        i += 1;
    }

    Ok(())
}

fn pwmp_msg_id_rng() -> MsgId {
    unsafe { esp_random() }
}
