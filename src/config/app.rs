use pwmp_client::pwmp_types::{multitype::SettingValue, setting::SettingName};
use std::time::Duration;

#[allow(clippy::struct_excessive_bools)]
pub struct AppConfig {
    pub battery_ignore: bool,
    pub ota: bool,
    pub sleep_time: Duration,
    pub sbop: bool,
    pub mute_notifications: bool,
}

impl AppConfig {
    pub const DEF_BATTERY_IGNORE: bool = false;
    pub const DEF_OTA: bool = false;
    pub const DEF_SLEEP_TIME: Duration = Duration::from_secs(60);
    pub const DEF_SBOP: bool = true;
    pub const DEF_MUTE_NOTIFICATIONS: bool = false;

    const N_SETTINGS: usize = 5;
    pub const ALL_SETTINGS: [SettingName; Self::N_SETTINGS] = [
        SettingName::BatteryIgnore,
        SettingName::Ota,
        SettingName::SleepTime,
        SettingName::Sbop,
        SettingName::MuteNotifications,
    ];

    pub fn update_settings(&mut self, values: [SettingValue; Self::N_SETTINGS]) {
        for (name, value) in Self::ALL_SETTINGS.iter().zip(values) {
            match name {
                SettingName::BatteryIgnore => self.battery_ignore = value.as_bool().unwrap(),
                SettingName::Ota => self.ota = value.as_bool().unwrap(),
                SettingName::Sbop => self.sbop = value.as_bool().unwrap(),
                SettingName::MuteNotifications => {
                    self.mute_notifications = value.as_bool().unwrap();
                }
                SettingName::SleepTime => {
                    self.sleep_time = Duration::from_secs(value.as_number().unwrap() as u64);
                }
                SettingName::DeviceSpecific => (),
            }
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            battery_ignore: Self::DEF_BATTERY_IGNORE,
            ota: Self::DEF_OTA,
            sleep_time: Self::DEF_SLEEP_TIME,
            sbop: Self::DEF_SBOP,
            mute_notifications: Self::DEF_MUTE_NOTIFICATIONS,
        }
    }
}
