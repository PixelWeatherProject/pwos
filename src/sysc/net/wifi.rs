use super::PowerSavingMode;
use crate::{
    os_debug,
    sysc::{OsError, OsResult, ReportableError},
};
use esp_idf_svc::{
    eventloop::{EspEventLoop, EspEventSource, EspSystemEventLoop, System, Wait},
    hal::modem::Modem,
    netif::IpEvent,
    nvs::EspDefaultNvsPartition,
    sys::{esp_wifi_set_max_tx_power, esp_wifi_set_ps, EspError},
    wifi::{
        AccessPointInfo, AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDeviceId,
        WifiEvent,
    },
};
use pwmp_client::pwmp_types::mac::Mac;
use std::time::Duration;

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
        let mut wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;

        wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;
        wifi.start()?;

        Ok(Self {
            driver: wifi,
            event_loop: sys_loop,
        })
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
        Ok(self.driver.scan_n::<MAXN>()?.0)
    }

    pub fn connect(
        &mut self,
        ssid: &str,
        psk: &str,
        auth: AuthMethod,
        timeout: Duration,
    ) -> OsResult<()> {
        self.driver
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.try_into().unwrap(),
                password: psk.try_into().unwrap(),
                auth_method: auth,
                ..Default::default()
            }))?;
        self.driver.connect().map_err(OsError::WifiConnect)?;

        // wait until connected
        self.await_event::<WifiEvent, _, _>(
            || self.driver.is_connected(),
            OsError::WifiConnect,
            timeout,
        )?;

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
