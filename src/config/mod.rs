use core::{iter::Iterator, option::Option};

mod app;
#[allow(clippy::doc_markdown)]
mod sys;

pub use app::{get_settings, save_settings};
pub use sys::*;

pub fn wifi_get_ap_psk(essid: &str) -> Option<&'static str> {
    sys::WIFI_NETWORKS
        .iter()
        .find(|(candidate_essid, _)| *candidate_essid == essid)
        .map(|(_, psk)| *psk)
}
