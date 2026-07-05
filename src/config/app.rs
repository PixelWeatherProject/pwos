//! This module contains functions for getting and saving the last known node settings (received from the PWMP server) into the RTC memory of the MCU.
//!
//! The settings are stored in a private mutable static variable. Generally, `mut static`s are unsafe, however:
//! - It's not possible to read/write from/to it directly, as it's private,
//! - A *copy* of this value can be retrieved using [`get_settings()`],
//! - It's value can be updated using [`save_settings()`].
//!
//! ## Warning
//! Calling [`save_settings()`] from multiple threads at the same time is unsafe and can cause a data-race.

use crate::sysc::rtcvar::RtcValue;
use pwmp_client::pwmp_msg::settings::NodeSettings;

/// Node application configuration.
#[link_section = ".rtc_noinit"]
static SETTINGS: RtcValue<NodeSettings> = RtcValue::new();

/// Get the last known node settings given by the PWMP server.
///
/// If no settings were saved before, the defaults are returned instead.
pub fn get_settings() -> NodeSettings {
    SETTINGS.read()
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn save_settings(settings: &NodeSettings) {
    SETTINGS.set(*settings);
}
