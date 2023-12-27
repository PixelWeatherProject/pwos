mod wifi;

use esp_idf_svc::sys::{
    wifi_ps_type_t_WIFI_PS_MAX_MODEM, wifi_ps_type_t_WIFI_PS_MIN_MODEM, wifi_ps_type_t_WIFI_PS_NONE,
};
pub use wifi::WiFi;

#[allow(unused)]
#[repr(u32)]
pub enum PowerSavingMode {
    Off = wifi_ps_type_t_WIFI_PS_NONE,
    Minimum = wifi_ps_type_t_WIFI_PS_MIN_MODEM,
    Maximum = wifi_ps_type_t_WIFI_PS_MAX_MODEM,
}
