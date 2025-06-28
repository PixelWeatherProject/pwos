//! This module contains functions for getting and saving the last known node settings (received from the PWMP server) into the RTC memory of the MCU.
//!
//! The settings are stored in a private mutable static variable. Generally, `mut static`s are unsafe, however:
//! - It's not possible to read/write from/to it directly, as it's private,
//! - A *copy* of this value can be retrieved using [`get_settings()`],
//! - It's value can be updated using [`save_settings()`].
//!
//! ## Warning
//! Calling [`save_settings()`] from multiple threads at the same time is unsafe and can cause a data-race.

use pwmp_client::pwmp_msg::settings::NodeSettings;

/// Node application configuration.
#[link_section = ".rtc.data"]
static mut SETTINGS: NodeSettings = NodeSettings::const_default();

/// Get the last known node settings given by the PWMP server.
///
/// If no settings were saved before, the defaults are returned instead.
pub fn get_settings() -> NodeSettings {
    // SAFETY: The static is not available directly and the firmware is not multithreaded.
    unsafe { SETTINGS }
}

pub fn save_settings(settings: &NodeSettings) {
    // SAFETY: The static is not available directly, and can be only retrieved using get_settings(), which returns only a copy.
    unsafe {
        SETTINGS.battery_ignore = settings.battery_ignore;
        SETTINGS.mute_notifications = settings.mute_notifications;
        SETTINGS.ota = settings.ota;
        SETTINGS.sbop = settings.sbop;
        SETTINGS.sleep_time = settings.sleep_time;
    }
}
