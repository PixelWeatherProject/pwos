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
            }
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        unsafe {
            Self {
                battery_ignore: SettingName::BatteryIgnore
                    .default_value()
                    .as_bool()
                    .unwrap_unchecked(),
                ota: SettingName::Ota
                    .default_value()
                    .as_bool()
                    .unwrap_unchecked(),
                sleep_time: Duration::from_secs(
                    SettingName::SleepTime
                        .default_value()
                        .as_number()
                        .unwrap_unchecked() as _,
                ),
                sbop: SettingName::Sbop
                    .default_value()
                    .as_bool()
                    .unwrap_unchecked(),
                mute_notifications: SettingName::MuteNotifications
                    .default_value()
                    .as_bool()
                    .unwrap_unchecked(),
            }
        }
    }
}
