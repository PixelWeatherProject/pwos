use crate::{
    config::{PWMP_SERVER, WIFI_NETWORKS, WIFI_TIMEOUT},
    re_esp,
    sysc::{
        self,
        battery::{Battery, CRITICAL_VOLTAGE},
        drivers::{AirPressureSensor, HumiditySensor, TemperatureSensor},
        ledctl::BoardLed,
        logging::GlobalOsLogger,
        net::wifi::{WiFi, RSSI_THRESHOLD},
        nvs::NonVolatileStorage,
        ota::{Ota, OtaHandle},
        power::{get_reset_reason, mcu_sleep, ResetReasonExt},
        usbctl, OsError, OsResult, ReportableError,
    },
};
use core::sync::atomic::{AtomicU8, Ordering};
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::modem::Modem, wifi::AccessPointInfo};
use pwmp_client::{
    ota::UpdateStatus,
    pwmp_msg::{artifact::Artifact, settings::NodeSettings, version::Version, MsgId},
    PwmpClient,
};

static PWMP_MSG_ID: AtomicU8 = AtomicU8::new(0);

#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
pub fn fw_main(
    mut battery: Battery,
    modem: Modem<'static>,
    sys_loop: EspSystemEventLoop,
    mut led: BoardLed,
    nvs: &NonVolatileStorage,
    ota: &mut Ota,
    cfg: &mut NodeSettings,
) -> OsResult<()> {
    if !ota.current_verified()? {
        log::warn!("Running unverified firmware");
    }

    let (wifi, ap, cached) = setup_wifi(modem, sys_loop)?;
    log::debug!("Connecting to PWMP");
    let mut pws =
        PwmpClient::new(PWMP_SERVER, &pwmp_msg_id_gen, None, None, None).inspect_err(|_| {
            // if the reused AP has changed it's IP configuration, we won't have internet access
            // this is not something that happens everyday, rather just a *VERY RARE* edge case
            // we signal failiure and attempt DHCP on next boot
            //
            // there are other ways to fix this but this is probably the "cheapest" performance
            // and memory wise
            if cached {
                log::error!("IP configuration may be outdated, clearing RTC cache");
                wifi.clear_config_cache();
            }
        })?;

    log::debug!("Sending handshake request");
    pws.initialize_session(wifi.get_mac()?, ota.current_version()?, None)?;

    log::debug!("Requesting app configuration");
    read_appcfg(&mut pws, cfg)?;

    log::debug!("Requesting date time");
    update_system_time(&mut pws)?;

    let int_temperature_sensor =
        TemperatureSensor::get_internal().ok_or(OsError::UnexpectedNull)?;
    let ext_temperature_sensor = TemperatureSensor::get_external().ok_or(OsError::NoEnvSensor)?;
    let humidity_sensor = HumiditySensor::get().ok_or(OsError::NoEnvSensor)?;
    let air_pressure_sensor = AirPressureSensor::get();

    let bat_voltage = battery.read()?;
    if usbctl::is_connected() {
        log::warn!("Battery voltage measurement may be affected by USB power");
        cfg.battery_ignore = true;
    }
    log::info!("Battery: {}.{:03}V", bat_voltage / 1000, bat_voltage % 1000);

    if (bat_voltage <= CRITICAL_VOLTAGE) && cfg.sbop && !cfg.battery_ignore {
        log::warn!("Battery voltage too low, activating sBOP");

        pws.send_notification("Battery voltage too low, activating sBOP")
            .report("Failed to send sBOP notification");

        mcu_sleep(None);
    }

    let rssi = wifi.live_rssi()?;
    let temperature = ext_temperature_sensor.read()?;
    let humidity = humidity_sensor.read()?;
    let air_pressure = match air_pressure_sensor {
        Some(s) => Some(s.read()?),
        None => None,
    };
    let cpu_die_temp = int_temperature_sensor.read()?;
    log::info!(
        "{}{}.{}*C / {humidity}%",
        if temperature < 0 { "-" } else { "" },
        temperature.unsigned_abs() / 10,
        temperature.unsigned_abs() % 10
    );

    log::debug!("Posting results");
    pws.post_measurements(
        temperature,
        humidity,
        air_pressure,
        bat_voltage,
        cpu_die_temp,
        &ap.ssid,
        rssi,
    )?;

    report_reset_reason(&mut pws);
    report_previous_run_error(&mut pws, nvs)?;
    report_ota_result(&mut pws, ota)?;
    ota_routine(&mut pws, ota)?;
    handle_diagnostics(&mut pws, cfg);

    pws.close_session(false)?;

    led.off();
    Ok(())
}

#[allow(clippy::similar_names)]
fn setup_wifi(
    modem: Modem<'static>,
    sys_loop: EspSystemEventLoop,
) -> OsResult<(WiFi, AccessPointInfo, bool)> {
    log::debug!("Starting WiFi setup");
    let mut wifi = WiFi::new(modem, sys_loop)?;

    match wifi_try_last(&mut wifi) {
        Ok((true, Some(ap))) => return Ok((wifi, ap, true)),
        Err(why) => return Err(why),
        _ => (),
    }

    log::debug!("Starting WiFi scan");
    #[cfg(debug_assertions)]
    let scan_start = std::time::Instant::now();
    let mut networks = wifi.scan()?;

    #[cfg(debug_assertions)]
    {
        let formatted_network_names = networks
            .iter()
            .map(|ap| format!("{} ({}dBm)", ap.ssid, ap.signal_strength))
            .collect::<Vec<String>>()
            .join(", ");

        log::debug!(
            "Found networks: {formatted_network_names} in {:.02?}",
            scan_start.elapsed()
        );
    }

    // filter out APs with RSSI >= RSSI_THRESHOLD
    networks.retain(|ap| ap.signal_strength >= RSSI_THRESHOLD);
    // sort by signal strength
    networks.sort_by_key(|b| std::cmp::Reverse(b.signal_strength));

    if networks.is_empty() {
        log::warn!("No usable networks found");
        return Err(OsError::NoInternet);
    }

    for ap in networks {
        match wifi_try_connect(&mut wifi, &ap, false) {
            Ok(()) => return Ok((wifi, ap, false)),
            Err(OsError::WifiPskNotFound) => (),
            Err(other) => log::error!("Failed to connect: {other}"),
        }
    }

    Err(OsError::NoInternet)
}

#[allow(clippy::similar_names)]
fn wifi_try_last(wifi: &mut WiFi) -> OsResult<(bool, Option<AccessPointInfo>)> {
    let last_ip = WiFi::last_ip();

    let Some(last_ap) = WiFi::last_ap() else {
        /* no network was stored from previous sessions */
        return Ok((false, None));
    };

    log::debug!("Reusing previous AP: {}", last_ap.ssid_as_str());
    let ap: AccessPointInfo = last_ap.into();

    if let Some(config) = last_ip {
        /* we have a cached DHCP configuration */
        log::debug!("Reusing previous IP configuration");
        wifi.set_static_ip_config(config)?;
    }

    if let Err(why) = wifi_try_connect(wifi, &ap, true) {
        /* failed to connect to cached AP */
        log::error!("Failed to connect to previous AP: {why}");

        // if we used static IP, reset the interface back to use DHCP
        if last_ip.is_some() {
            wifi.set_dymanic_ip_config()?;
        }

        // signal the called that we could not reuse the previous AP
        return Ok((false, None));
    }

    /* successfully connected */
    Ok((true, Some(ap)))
}

fn wifi_try_connect(wifi: &mut WiFi, ap: &AccessPointInfo, skip_save: bool) -> OsResult<()> {
    let psk = WIFI_NETWORKS
        .iter()
        .find(|candidate| candidate.0 == ap.ssid)
        .map(|candidate| candidate.1)
        .ok_or(OsError::WifiPskNotFound)?;

    log::debug!("Connecting to {} ({}dBm)", ap.ssid, ap.signal_strength);

    let start = std::time::Instant::now();
    match wifi.connect(ap, psk, WIFI_TIMEOUT, skip_save) {
        Ok(()) => {
            log::debug!("Connected in {:.02?}", start.elapsed());
            log::debug!("IP: {}", wifi.get_ip_info()?.ip);
            Ok(())
        }
        Err(why) => Err(why),
    }
}

fn report_reset_reason(pws: &mut PwmpClient) {
    let reset_reason = get_reset_reason();
    if reset_reason.is_abnormal() {
        log::warn!("Detected abnormal reset reason: {reset_reason:?}");

        pws.send_notification(format!("Detected abnormal reset reason: {reset_reason:?}"))
            .report("Failed to report abnormal reset reason");
    }
}

fn report_previous_run_error(pws: &mut PwmpClient, nvs: &NonVolatileStorage) -> OsResult<()> {
    if let Some(error) = nvs.get_last_os_error()? {
        log::info!("Reporting error from previous run ({error})");

        pws.send_notification(format!("Error from previous run: {error}"))
            .report("Failed to report previous error");

        nvs.clear_last_os_error()?;
    } else {
        log::debug!("No error detected from previous run");
    }

    Ok(())
}

fn report_ota_result(pws: &mut PwmpClient, ota: &Ota) -> OsResult<()> {
    if ota.report_needed()? {
        let success = !ota.rollback_detected()?;

        log::info!(
            "Reporting {}successfull firmware update",
            if success { "" } else { "un" }
        );

        pws.send_notification(format!(
            "Update to PWOS {} has {}",
            if success {
                env!("CARGO_PKG_VERSION").to_string()
            } else {
                ota.previous_version()?.to_string()
            },
            if success { "succeeded" } else { "failed" }
        ))?;

        pws.report_firmware(success)?;
        ota.mark_reported();
    } else {
        log::debug!("No update report needed");
    }
    Ok(())
}

fn ota_routine(pws: &mut PwmpClient, ota: &mut Ota) -> OsResult<()> {
    log::debug!("Checking for updates");
    if check_ota(pws)? {
        let mut handle = ota.begin_update()?;

        if let Err(why) = begin_update(pws, &mut handle) {
            log::error!("OTA failed: {why}, aborting");
            handle.cancel()?;
        }

        log::info!("Update installed successfully");
    } // Handle will be dropped and the update should finalize

    Ok(())
}

fn handle_diagnostics(pws: &mut PwmpClient, appcfg: &NodeSettings) {
    if !appcfg.diagnostics {
        return;
    }

    if let Some(logs) = GlobalOsLogger::load_history() {
        log::debug!("Sending diagnostic logs");
        pws.send_diagnostic_data("logs", Artifact::String(logs))
            .report("Failed to send log history");
    } else {
        log::debug!("No diagnostic logs were saved");
    }
}

fn read_appcfg(pws: &mut PwmpClient, appcfg: &mut NodeSettings) -> OsResult<()> {
    log::debug!("Reading settings");

    if let Some(settings) = pws.get_settings()? {
        *appcfg = settings;
        crate::config::save_settings(appcfg);
    } else {
        log::warn!("Got empty node settings, using defaults");
    }

    log::debug!("Settings updated");
    Ok(())
}

fn update_system_time(pws: &mut PwmpClient) -> OsResult<()> {
    if !sysc::time::resync_recommended()? {
        log::debug!("Wall clock in sync");
        return Ok(());
    }

    log::debug!("Wall clock not set or out of sync");

    let epoch = pws.get_date_time(true)?;
    log::debug!("Got epoch: {epoch}");

    sysc::time::set_system_time(epoch)?;
    log::debug!("Wall clock time updated");

    Ok(())
}

fn check_ota(pws: &mut PwmpClient) -> OsResult<bool> {
    let current_version =
        Version::parse(env!("CARGO_PKG_VERSION")).ok_or(OsError::IllegalFirmwareVersion)?;

    match pws.check_os_update(current_version)? {
        UpdateStatus::UpToDate => {
            log::info!("No update available");
            Ok(false)
        }
        UpdateStatus::Available(new_version) => {
            log::info!("Update v{new_version} available");
            Ok(true)
        }
    }
}

fn begin_update(pws: &mut PwmpClient, handle: &mut OtaHandle) -> OsResult<()> {
    const CHUNK_SIZE: u32 = 4096;
    let mut maybe_chunk = pws.next_update_chunk(Some(CHUNK_SIZE))?;

    while let Some(chunk) = maybe_chunk {
        log::debug!("Writing next OTA update chunk");

        re_esp!(handle.write(&chunk), OtaWrite)?;
        maybe_chunk = pws.next_update_chunk(Some(CHUNK_SIZE))?;
    }

    Ok(())
}

/// Simple message ID generator for PWMP messages.
fn pwmp_msg_id_gen() -> MsgId {
    PWMP_MSG_ID.fetch_add(1, Ordering::Relaxed)
}
