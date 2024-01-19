use super::PowerSavingMode;
use crate::{
    os_debug,
    sysc::{OsError, OsResult, ReportableError},
    wrap_oserr,
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    sys::{esp_wifi_set_max_tx_power, esp_wifi_set_ps, EspError},
    wifi::{
        AccessPointInfo, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
        WifiDeviceId,
    },
};
use pwmp_client::pwmp_types::mac::Mac;

pub struct WiFi(BlockingWifi<EspWifi<'static>>);

#[allow(clippy::unused_self)]
impl WiFi {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        nvs: EspDefaultNvsPartition,
    ) -> OsResult<Self> {
        let esp_wifi = wrap_oserr!(EspWifi::new(modem, sys_loop.clone(), Some(nvs)), WifiInit)?;
        let mut wifi = wrap_oserr!(BlockingWifi::wrap(esp_wifi, sys_loop), WifiBlockingInit)?;

        wrap_oserr!(
            wifi.set_configuration(&Configuration::Client(ClientConfiguration::default())),
            WifiConfig
        )?;
        wrap_oserr!(wifi.start(), WifiStart)?;

        Ok(Self(wifi))
    }

    pub fn set_power_saving(&self, mode: PowerSavingMode) -> OsResult<()> {
        EspError::convert(unsafe { esp_wifi_set_ps(mode as u32) })?;
        Ok(())
    }

    pub fn set_power(&self, pow: u8) -> OsResult<()> {
        assert!(
            (8..=84).contains(&pow),
            "Power outside allowed range <8;84>"
        );

        EspError::convert(unsafe {
            esp_wifi_set_max_tx_power(i8::try_from(pow).unwrap_unchecked())
        })?;

        Ok(())
    }

    pub fn scan<const MAXN: usize>(&mut self) -> OsResult<heapless::Vec<AccessPointInfo, MAXN>> {
        Ok(self.0.scan_n::<MAXN>()?.0)
    }

    pub fn connect(&mut self, ssid: &str, psk: &str, auth: AuthMethod) -> OsResult<()> {
        wrap_oserr!(
            self.0
                .set_configuration(&Configuration::Client(ClientConfiguration {
                    ssid: ssid.into(),
                    password: psk.into(),
                    auth_method: auth,
                    ..Default::default()
                })),
            WifiConfig
        )?;

        wrap_oserr!(self.0.connect(), WifiConnect)?;
        self.0.wait_netif_up()?;

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn get_ip_info(&self) -> OsResult<esp_idf_svc::ipv4::IpInfo> {
        Ok(self.0.wifi().sta_netif().get_ip_info()?)
    }

    pub fn get_mac(&self) -> OsResult<Mac> {
        let raw = self.0.wifi().get_mac(WifiDeviceId::Sta)?;

        Ok(Mac::new(raw[0], raw[1], raw[2], raw[3], raw[4], raw[5]))
    }

    fn connected(&self) -> bool {
        self.0.is_connected().unwrap_or(false)
    }
}

impl Drop for WiFi {
    fn drop(&mut self) {
        os_debug!("Deinitializing WiFi");

        if self.connected() {
            self.0.disconnect().report("Failed to disconnect");
        }

        self.0.stop().report("Failed to disable");
    }
}
