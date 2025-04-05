use crate::{
    config::{STATIC_IP_CONFIG, WIFI_COUNTRY_CODE},
    null_check, os_debug,
    sysc::{OsError, OsResult},
};
use esp_idf_svc::{
    eventloop::{EspEventLoop, EspEventSource, EspSystemEventLoop, System, Wait},
    hal::modem::Modem,
    ipv4::{
        ClientConfiguration as IpClientConfiguration, Configuration as IpConfiguration,
        DHCPClientSettings,
    },
    netif::{EspNetif, IpEvent, NetifConfiguration},
    sys::{
        esp, esp_wifi_scan_start, esp_wifi_set_country_code, esp_wifi_set_storage,
        wifi_storage_t_WIFI_STORAGE_RAM, EspError,
    },
    wifi::{
        AccessPointInfo, AuthMethod, ClientConfiguration, Configuration, EspWifi, PmfConfiguration,
        ScanMethod, WifiDeviceId, WifiDriver, WifiEvent,
    },
};
use pwmp_client::pwmp_msg::{aliases::Rssi, mac::Mac};
use std::{fmt::Write, ptr, time::Duration};

/// Maximum number of networks to scan
pub const MAX_NET_SCAN: usize = 2;

/// Maximum acceptable signal strength
pub const RSSI_THRESHOLD: Rssi = -85;

pub struct WiFi {
    driver: EspWifi<'static>,
    event_loop: EspEventLoop<System>,
}

#[allow(clippy::unused_self)]
impl WiFi {
    pub fn new(modem: Modem, sys_loop: EspSystemEventLoop) -> OsResult<Self> {
        let wifi = WifiDriver::new(modem, sys_loop.clone(), None)?;
        let ip_config = if STATIC_IP_CONFIG.is_some() {
            Self::generate_static_ip_config()
        } else {
            Self::generate_dhcp_config(&wifi)
        }?;

        esp!(unsafe { esp_wifi_set_storage(wifi_storage_t_WIFI_STORAGE_RAM) })?;

        os_debug!("Configuring WiFi interface");
        let mut wifi = EspWifi::wrap_all(wifi, EspNetif::new_with_conf(&ip_config)?)?;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

        os_debug!("Starting WiFi interface");
        wifi.start()?;

        os_debug!("Setting country code");
        esp!(unsafe { esp_wifi_set_country_code(WIFI_COUNTRY_CODE.as_ptr().cast(), true) })?;

        Ok(Self {
            driver: wifi,
            event_loop: sys_loop,
        })
    }

    pub fn scan(&mut self) -> OsResult<heapless::Vec<AccessPointInfo, MAX_NET_SCAN>> {
        // Due to a bug in `esp-idf-svc` causing `ScanModes` to not be properly converted
        // to `wifi_scan_type_t_*` this alternative is faster.
        // Default scan configuration is documented here: https://docs.espressif.com/projects/esp-idf/en/v5.3.2/esp32/api-reference/network/esp_wifi.html?#_CPPv419esp_wifi_scan_startPK18wifi_scan_config_tb
        esp!(unsafe {
            esp_wifi_scan_start(ptr::null() /* intentional */, true)
        })?;

        // Since the scan is blocking, we don't need to wait until it finishes.
        Ok(self.driver.get_scan_result_n()?.0)
    }

    pub fn connect(
        &mut self,
        ssid: &heapless::String<32>,
        bssid: &[u8; 6],
        channel: u8,
        psk: &str,
        auth: AuthMethod,
        timeout: Duration,
    ) -> OsResult<()> {
        self.driver
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.clone(),
                password: psk.try_into().map_err(|()| OsError::ArgumentTooLong)?,
                auth_method: auth,
                scan_method: ScanMethod::FastScan, // https://github.com/espressif/esp-idf/tree/master/examples/wifi/fast_scan
                /* the following parameters may improve connection times */
                bssid: Some(*bssid),
                channel: Some(channel),
                pmf_cfg: PmfConfiguration::NotCapable, /* disables IEEE 802.11w-2009 support */
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

    fn generate_dhcp_config(wifi_driver: &WifiDriver) -> OsResult<NetifConfiguration> {
        Ok(NetifConfiguration {
            ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::DHCP(
                DHCPClientSettings {
                    hostname: Some(Self::generate_hostname(wifi_driver)?),
                },
            ))),
            ..NetifConfiguration::wifi_default_client()
        })
    }

    fn generate_static_ip_config() -> OsResult<NetifConfiguration> {
        Ok(NetifConfiguration {
            ip_configuration: Some(IpConfiguration::Client(IpClientConfiguration::Fixed(
                null_check!(STATIC_IP_CONFIG),
            ))),

            ..NetifConfiguration::wifi_default_client()
        })
    }

    fn generate_hostname(wifi_driver: &WifiDriver) -> OsResult<heapless::String<30>> {
        let mut buffer = heapless::String::new();
        let last_two_bytes = &wifi_driver.get_mac(WifiDeviceId::Sta).unwrap_or_default()[4..6];

        write!(
            &mut buffer,
            "pixelweather-node-{:02X}{:02X}",
            last_two_bytes[0], last_two_bytes[1]
        )
        .map_err(|_| OsError::UnexpectedBufferFailiure)?;

        Ok(buffer)
    }
}
