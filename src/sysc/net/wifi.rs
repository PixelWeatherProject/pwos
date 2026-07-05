use crate::{
    config::WIFI_COUNTRY_CODE,
    re_esp,
    sysc::{
        hash,
        rtcvar::{RtcObject, RtcValue},
        OsError, OsResult,
    },
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
        esp, esp_wifi_scan_start, esp_wifi_set_country_code, esp_wifi_set_max_tx_power,
        esp_wifi_set_ps, esp_wifi_set_storage, wifi_ps_type_t_WIFI_PS_NONE,
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

/// Last used access point
#[link_section = ".rtc_noinit"]
static LAST_AP: RtcValue<Option<ApMetadata>> = RtcValue::new();

/// A simpler version of [`AccessPointInfo`].
#[derive(Default)]
pub struct ApMetadata {
    ssid: [u8; 32],
    bssid: [u8; 6],
    channel: u8,
    signal_strength: i8, // technically we dont need this but it is used in a log message
    auth_method: Option<AuthMethod>,
}

pub struct WiFi {
    driver: EspWifi<'static>,
    event_loop: EspEventLoop<System>,
}

#[allow(clippy::unused_self)]
impl WiFi {
    pub fn new(modem: Modem<'static>, sys_loop: EspSystemEventLoop) -> OsResult<Self> {
        let wifi = re_esp!(WifiDriver::new(modem, sys_loop.clone(), None), WifiInit)?;
        let ip_config = Self::generate_dhcp_config(&wifi)?;

        re_esp!(
            esp!(unsafe { esp_wifi_set_storage(wifi_storage_t_WIFI_STORAGE_RAM) }),
            WifiParam
        )?;

        log::debug!("Configuring WiFi interface");
        let sta_netif = re_esp!(EspNetif::new_with_conf(&ip_config), WifiInit)?;
        let mut wifi = re_esp!(EspWifi::wrap_all(wifi, sta_netif), WifiInit)?;
        re_esp!(
            wifi.set_configuration(&Configuration::Client(ClientConfiguration::default())),
            WifiConfig
        )?;

        log::debug!("Starting WiFi interface");
        re_esp!(wifi.start(), WifiStart)?;

        log::debug!("Setting hardware parameters");
        unsafe {
            re_esp!(
                esp!(esp_wifi_set_country_code(
                    WIFI_COUNTRY_CODE.as_ptr().cast(),
                    true
                )),
                WifiParam
            )?;

            re_esp!(esp!(esp_wifi_set_max_tx_power(84)), WifiParam)?;

            re_esp!(
                esp!(esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE)),
                WifiParam
            )?;
        }

        Ok(Self {
            driver: wifi,
            event_loop: sys_loop,
        })
    }

    pub fn scan(&mut self) -> OsResult<heapless::Vec<AccessPointInfo, MAX_NET_SCAN>> {
        // Due to a bug in `esp-idf-svc` causing `ScanModes` to not be properly converted
        // to `wifi_scan_type_t_*` this alternative is faster.
        // Default scan configuration is documented here: https://docs.espressif.com/projects/esp-idf/en/v5.3.2/esp32/api-reference/network/esp_wifi.html?#_CPPv419esp_wifi_scan_startPK18wifi_scan_config_tb
        re_esp!(
            esp!(unsafe {
                esp_wifi_scan_start(ptr::null() /* intentional */, true)
            }),
            WifiScan
        )?;

        // Since the scan is blocking, we don't need to wait until it finishes.
        Ok(re_esp!(self.driver.get_scan_result_n(), WifiScan)?.0)
    }

    pub fn connect(&mut self, ap: &AccessPointInfo, psk: &str, timeout: Duration) -> OsResult<()> {
        re_esp!(
            self.driver
                .set_configuration(&Configuration::Client(ClientConfiguration {
                    ssid: ap.ssid.clone(),
                    password: heapless::String::<64>::try_from(psk)
                        .map_err(|_| OsError::ArgumentTooLong)?,
                    auth_method: ap.auth_method.unwrap_or_default(),
                    scan_method: ScanMethod::FastScan, // https://github.com/espressif/esp-idf/tree/master/examples/wifi/fast_scan
                    /* the following parameters may improve connection times */
                    bssid: Some(ap.bssid),
                    channel: Some(ap.channel),
                    pmf_cfg: PmfConfiguration::NotCapable, /* disables IEEE 802.11w-2009 support */
                })),
            WifiConfig
        )?;
        log::debug!("Starting connection to AP");
        self.driver.connect().map_err(OsError::WifiConnect)?;

        log::debug!("Waiting for connection result");
        // wait until connected
        self.await_event::<WifiEvent, _, _>(
            || self.driver.is_connected(),
            OsError::EventTimeout,
            timeout,
        )?;

        log::debug!("Waiting for IP address");
        // wait until we get an IP
        self.await_event::<IpEvent, _, _>(|| self.driver.is_up(), OsError::EventTimeout, timeout)?;

        log::debug!("Saving AP");
        LAST_AP.set(Some(ap.into()));

        Ok(())
    }

    pub fn last_ap(&self) -> Option<ApMetadata> {
        LAST_AP.read()
    }

    pub fn get_ip_info(&self) -> OsResult<esp_idf_svc::ipv4::IpInfo> {
        Ok(re_esp!(self.driver.sta_netif().get_ip_info(), WifiInfo)?)
    }

    pub fn get_mac(&self) -> OsResult<Mac> {
        let raw = re_esp!(self.driver.get_mac(WifiDeviceId::Sta), WifiInfo)?;

        Ok(Mac::new(raw[0], raw[1], raw[2], raw[3], raw[4], raw[5]))
    }

    fn await_event<S, F, U>(&self, matcher: F, err_map: U, timeout: Duration) -> OsResult<()>
    where
        S: EspEventSource,
        F: Fn() -> Result<bool, EspError>,
        U: Fn(EspError) -> OsError,
    {
        let wait = re_esp!(Wait::new::<S>(&self.event_loop), EventWaiterInit)?;
        wait.wait_while(|| matcher().map(|s| !s), Some(timeout))
            .map_err(err_map)
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

impl ApMetadata {
    pub fn ssid_as_str(&self) -> &str {
        let end = self.ssid.into_iter().position(|c| c == 0).unwrap_or(31);
        unsafe { std::str::from_utf8_unchecked(&self.ssid[..end]) }
    }
}

impl From<&AccessPointInfo> for ApMetadata {
    fn from(value: &AccessPointInfo) -> Self {
        let mut ssid = [0; 32];
        ssid[..value.ssid.len()].copy_from_slice(value.ssid.as_bytes());

        Self {
            ssid,
            bssid: value.bssid,
            channel: value.channel,
            signal_strength: value.signal_strength,
            auth_method: value.auth_method,
        }
    }
}

impl From<ApMetadata> for AccessPointInfo {
    fn from(value: ApMetadata) -> Self {
        let mut ssid: heapless::String<32> = heapless::String::new();

        for byte in value.ssid {
            if byte == 0 {
                break;
            }
            let _ = ssid.push(char::from(byte));
        }

        Self {
            ssid,
            bssid: value.bssid,
            channel: value.channel,
            signal_strength: value.signal_strength,
            auth_method: value.auth_method,
            // lossy conversion but these are never used in `connect()`
            ..Default::default()
        }
    }
}

impl RtcObject for ApMetadata {
    fn checksum(&self) -> u32 {
        let mut bytes = [0; 40];

        bytes[0..=31].copy_from_slice(&self.ssid);
        bytes[32..=37].copy_from_slice(&self.bssid);
        bytes[38] = self.channel;
        bytes[39] = self.auth_method.map_or(0, |val| match val {
            AuthMethod::None => 1,
            AuthMethod::WEP => 2,
            AuthMethod::WPA => 3,
            AuthMethod::WPA2Personal => 4,
            AuthMethod::WPAWPA2Personal => 5,
            AuthMethod::WPA2Enterprise => 6,
            AuthMethod::WPA3Personal => 7,
            AuthMethod::WPA2WPA3Personal => 8,
            AuthMethod::WAPIPersonal => 9,
        });

        hash::crc32(&bytes)
    }
}
