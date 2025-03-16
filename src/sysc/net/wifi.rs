use super::PowerSavingMode;
use crate::{
    config::{STATIC_IP_CONFIG, WIFI_COUNTRY_CODE},
    os_debug,
    sysc::{OsError, OsResult, ReportableError},
};
use esp_idf_svc::{
    eventloop::{EspEventLoop, EspEventSource, EspSystemEventLoop, System, Wait},
    hal::modem::Modem,
    ipv4::{
        ClientConfiguration as IpClientConfiguration, Configuration as IpConfiguration,
        DHCPClientSettings,
    },
    netif::{EspNetif, IpEvent, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    sys::{
        esp, esp_wifi_set_country_code, esp_wifi_set_ps, esp_wifi_set_storage,
        wifi_storage_t_WIFI_STORAGE_RAM, EspError,
    },
    wifi::{
        config::{ScanConfig, ScanType},
        AccessPointInfo, AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDeviceId,
        WifiDriver, WifiEvent,
    },
};
use pwmp_client::pwmp_msg::mac::Mac;
use std::{thread::sleep, time::Duration};

/// Maximum number of networks to scan
pub const MAX_NET_SCAN: usize = 2;

pub struct WiFi {
    driver: EspWifi<'static>,
    event_loop: EspEventLoop<System>,
}

#[allow(clippy::unused_self)]
impl WiFi {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
    ) -> OsResult<Self> {
        let wifi = WifiDriver::new(modem, sys_loop.clone(), Some(nvs))?;
        let ip_config = if STATIC_IP_CONFIG.is_some() {
            Self::generate_static_ip_config()
        } else {
            Self::generate_dhcp_config(&wifi)
        };

        os_debug!("Configuring WiFi interface");
        let mut wifi = EspWifi::wrap_all(
            wifi,
            EspNetif::new_with_conf(&ip_config)?,
            EspNetif::new(NetifStack::Ap)?,
        )?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

        esp!(unsafe { esp_wifi_set_storage(wifi_storage_t_WIFI_STORAGE_RAM) })?;

        os_debug!("Starting WiFi interface");
        wifi.start()?;

        os_debug!("Setting country code");
        esp!(unsafe { esp_wifi_set_country_code(WIFI_COUNTRY_CODE.as_ptr().cast(), true) })?;

        Ok(Self {
            driver: wifi,
            event_loop: sys_loop,
        })
    }

    pub fn set_power_saving(&self, mode: PowerSavingMode) -> OsResult<()> {
        EspError::convert(unsafe { esp_wifi_set_ps(mode as u32) })?;
        Ok(())
    }

    pub fn scan(
        &mut self,
        timeout: Duration,
    ) -> OsResult<heapless::Vec<AccessPointInfo, MAX_NET_SCAN>> {
        self.driver.start_scan(
            &ScanConfig {
                bssid: None,
                ssid: None,
                channel: None,
                scan_type: ScanType::Passive(timeout),
                show_hidden: false,
            },
            false,
        )?;

        sleep(timeout);
        self.driver.stop_scan()?;

        Ok(self.driver.get_scan_result_n()?.0)
    }

    pub fn connect(
        &mut self,
        ssid: &str,
        psk: &str,
        auth: AuthMethod,
        timeout: Duration,
    ) -> OsResult<()> {
        if ssid.len() > 32 {
            return Err(OsError::SsidTooLong);
        }

        if psk.len() > 64 {
            return Err(OsError::PskTooLong);
        }

        self.driver
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: unsafe { ssid.try_into().unwrap_unchecked() },
                password: unsafe { psk.try_into().unwrap_unchecked() },
                auth_method: auth,
                ..Default::default()
            }))?;
        os_debug!("Starting connection to AP");
        self.driver.connect().map_err(OsError::WifiConnect)?;

        os_debug!("Waiting for connection result");
        // wait until connected
        self.await_event::<WifiEvent, _, _>(
            || self.driver.is_connected(),
            OsError::WifiConnect,
            timeout,
        )?;

        if STATIC_IP_CONFIG.is_some() {
            os_debug!("Static IP configuration detected, skipping wait for IP address");
            return Ok(());
        }

        os_debug!("Waiting for IP address");
        // wait until we get an IP
        self.await_event::<IpEvent, _, _>(|| self.driver.is_up(), OsError::WifiConnect, timeout)?;

        Ok(())
    }

    fn await_event<S, F, U>(&self, matcher: F, err_map: U, timeout: Duration) -> OsResult<()>
    where
        S: EspEventSource,
        F: Fn() -> Result<bool, EspError>,
        U: Fn(EspError) -> OsError,
    {
        let wait = Wait::new::<S>(&self.event_loop)?;
        wait.wait_while(|| matcher().map(|s| !s), Some(timeout))
            .map_err(err_map)
    }

    #[cfg(debug_assertions)]
    pub fn get_ip_info(&self) -> OsResult<esp_idf_svc::ipv4::IpInfo> {
        Ok(self.driver.sta_netif().get_ip_info()?)
    }

    pub fn get_mac(&self) -> OsResult<Mac> {
        let raw = self.driver.get_mac(WifiDeviceId::Sta)?;

        Ok(Mac::new(raw[0], raw[1], raw[2], raw[3], raw[4], raw[5]))
    }

    fn connected(&self) -> bool {
        self.driver.is_connected().unwrap_or(false)
    }

    fn generate_dhcp_config(wifi_driver: &WifiDriver) -> NetifConfiguration {
        NetifConfiguration {
            ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::DHCP(
                DHCPClientSettings {
                    hostname: Some(Self::generate_hostname(wifi_driver)),
                },
            ))),
            ..NetifConfiguration::wifi_default_client()
        }
    }

    fn generate_static_ip_config() -> NetifConfiguration {
        NetifConfiguration {
            ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::Fixed(
                unsafe { STATIC_IP_CONFIG.unwrap_unchecked() },
            ))),

            ..NetifConfiguration::wifi_default_client()
        }
    }

    fn generate_hostname(wifi_driver: &WifiDriver) -> heapless::String<30> {
        let mut buffer = heapless::String::new();
        let last_two_bytes = &wifi_driver.get_mac(WifiDeviceId::Sta).unwrap_or_default()[4..6];

        // SAFETY: This is less than 30 characters, so it will always fit.
        unsafe {
            buffer.push_str("pixelweather-node-").unwrap_unchecked(); // 18 chars
            buffer
                .push_str(&format!("{last_two_bytes:02X?}"))
                .unwrap_unchecked(); // max 4 characters
        }

        buffer
    }
}

impl Drop for WiFi {
    fn drop(&mut self) {
        os_debug!("Deinitializing WiFi");

        if self.connected() {
            self.driver.disconnect().report("Failed to disconnect");
        }

        self.driver.stop().report("Failed to disable");
    }
}
