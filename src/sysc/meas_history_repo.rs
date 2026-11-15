//! A measurement history repository implementation.
//!
//! This module contains implementations for a reposity that stores measurement data from
//! sensors in RTC memory which were collected during runs where the node was offile or was
//! otherwise unable to post the data to the PWMP server.
//!
//! The repository is backed by a ring buffer, which collects up to `N` amount if data, and
//! when the device goes online successfully will then read the whole buffer and post all
//! measurements. The buffer is then cleared.

use crate::sysc::rtcvar::{ringbuf::RtcRingBuffer, RtcValue};
use pwmp_client::pwmp_msg::measurement::{HistoricMeasurement, MeasurementCollection};

/// Maximum amount of measurements remembered.
const HISTORY_SIZE: usize = 32;

#[link_section = ".rtc_noinit"]
static MEASUREMENT_HISTORY: RtcValue<RtcRingBuffer<HistoricMeasurement, HISTORY_SIZE>> =
    RtcValue::new();

pub struct MeasurementHistoryRepository;

impl MeasurementHistoryRepository {
    pub fn insert(measurement: MeasurementCollection) {
        let historic = HistoricMeasurement::new_now(measurement);

        MEASUREMENT_HISTORY.update(|buffer| {
            buffer.insert(historic);
        });
    }

    pub fn is_empy() -> bool {
        MEASUREMENT_HISTORY.get().is_empty()
    }

    pub fn get_all() -> Box<[HistoricMeasurement]> {
        MEASUREMENT_HISTORY.get_ref().collect()
    }

    pub fn clear() {
        MEASUREMENT_HISTORY.update(RtcRingBuffer::clear);
    }
}
