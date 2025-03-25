use pwmp_client::pwmp_msg::settings::NodeSettings;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

pub struct AppConfig(NodeSettings);

impl AppConfig {
    pub const fn sleep_time(&self) -> Duration {
        Duration::from_secs(self.0.sleep_time as _)
    }
}

impl Deref for AppConfig {
    type Target = NodeSettings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self(NodeSettings {
            battery_ignore: false,
            ota: true,
            sleep_time: 60,
            sbop: true,
            mute_notifications: false,
        })
    }
}
