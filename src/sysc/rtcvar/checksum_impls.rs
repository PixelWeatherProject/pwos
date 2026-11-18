use super::RtcObject;
use crate::sysc::hash;
use esp_idf_svc::ipv4::{self, Mask, Subnet};
use pwmp_client::pwmp_msg::{
    measurement::{HistoricMeasurement, MeasurementCollection},
    settings::NodeSettings,
};
use std::net::Ipv4Addr;

impl RtcObject for bool {
    fn checksum(&self) -> u32 {
        hash::crc32(&[u8::from(*self)])
    }

    fn new_empty() -> Self {
        Self::default()
    }
}

impl RtcObject for u8 {
    fn checksum(&self) -> u32 {
        hash::crc32(&[*self])
    }

    fn new_empty() -> Self {
        Self::default()
    }
}

impl RtcObject for u64 {
    fn checksum(&self) -> u32 {
        hash::crc32(self.to_ne_bytes().as_slice())
    }

    fn new_empty() -> Self {
        Self::default()
    }
}

impl RtcObject for NodeSettings {
    fn checksum(&self) -> u32 {
        let mut raw = [0; 6];

        raw[0] = u8::from(self.battery_ignore);
        raw[1] = u8::from(self.ota);
        raw[2..=3].copy_from_slice(&self.sleep_time.to_ne_bytes());
        raw[4] = u8::from(self.sbop);
        raw[5] = u8::from(self.mute_notifications);

        hash::crc32(&raw)
    }

    fn new_empty() -> Self {
        Self::default()
    }
}

impl RtcObject for ipv4::Ipv4Addr {
    fn checksum(&self) -> u32 {
        hash::crc32(&self.octets())
    }

    fn new_empty() -> Self {
        Self::UNSPECIFIED
    }
}

impl RtcObject for ipv4::ClientSettings {
    fn checksum(&self) -> u32 {
        // ip - 4 bytes
        // subnet
        //  - gateway ip - 4 bytes
        //  - mask - 1 byte
        // dns - 4 bytes
        // secondary dns - 4 bytes

        let mut raw = [0; 19];

        raw[0..=3].copy_from_slice(&self.ip.octets());
        raw[4..=7].copy_from_slice(&self.subnet.gateway.octets());
        raw[8] = self.subnet.mask.0;

        if let Some(ip) = self.dns {
            raw[9] = 1;
            raw[10..=13].copy_from_slice(&ip.octets());
        }
        if let Some(ip) = self.secondary_dns {
            raw[14] = 1;
            raw[15..=18].copy_from_slice(&ip.octets());
        }

        hash::crc32(&raw)
    }

    fn new_empty() -> Self {
        Self {
            ip: Ipv4Addr::UNSPECIFIED,
            subnet: Subnet {
                gateway: Ipv4Addr::UNSPECIFIED,
                mask: Mask(0),
            },
            dns: Some(Ipv4Addr::UNSPECIFIED),
            secondary_dns: None,
        }
    }
}

impl RtcObject for MeasurementCollection {
    fn checksum(&self) -> u32 {
        // temperature  - 2 bytes (i16)
        // humidity     - 1 byte  (u8)
        // air pressure - 1 byte presence flag + 2 bytes (Option<u16>)
        // battery      - 2 bytes (u16)
        // cpu temp     - 2 bytes (i16)
        let mut raw = [0; 10];

        raw[0..=1].copy_from_slice(&self.temperature.to_ne_bytes());
        raw[2] = self.humidity;

        if let Some(air_pressure) = self.air_pressure {
            raw[3] = 1;
            raw[4..=5].copy_from_slice(&air_pressure.to_ne_bytes());
        }

        raw[6..=7].copy_from_slice(&self.battery.to_ne_bytes());
        raw[8..=9].copy_from_slice(&self.cpu_temp.to_ne_bytes());

        hash::crc32(&raw)
    }

    fn new_empty() -> Self {
        Self {
            temperature: 0,
            humidity: 0,
            air_pressure: None,
            battery: 0,
            cpu_temp: 0,
        }
    }
}

impl RtcObject for HistoricMeasurement {
    fn checksum(&self) -> u32 {
        // collection checksum - 4 bytes (u32)
        // timestamp           - 8 bytes (u64)
        let mut raw = [0; 12];

        raw[0..=3].copy_from_slice(&self.collection().checksum().to_ne_bytes());
        raw[4..=11].copy_from_slice(&self.timestamp().to_ne_bytes());

        hash::crc32(&raw)
    }

    fn new_empty() -> Self {
        Self::new(MeasurementCollection::new_empty(), 0)
    }
}

impl<T: RtcObject> RtcObject for Option<T> {
    fn checksum(&self) -> u32 {
        self.as_ref()
            .map_or_else(|| hash::crc32(&[0]), RtcObject::checksum)
    }

    fn new_empty() -> Self {
        None
    }
}

impl<const N: usize> RtcObject for super::string::RtcString<N> {
    fn checksum(&self) -> u32 {
        hash::crc32(self.as_bytes())
    }

    fn new_empty() -> Self {
        Self::new()
    }
}

impl<T: RtcObject, const N: usize> RtcObject for super::array::RtcArray<T, N> {
    fn checksum(&self) -> u32 {
        self.crc32_hash()
    }

    fn new_empty() -> Self {
        Self::const_new(T::new_empty())
    }
}

impl<T: RtcObject, const N: usize> RtcObject for super::fifo::RtcFifo<T, N> {
    fn checksum(&self) -> u32 {
        self.crc32_hash()
    }

    fn new_empty() -> Self {
        Self::const_new(T::new_empty())
    }
}
