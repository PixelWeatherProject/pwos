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
