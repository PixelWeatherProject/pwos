use super::RtcObject;
use crate::sysc::hash;
use esp_idf_svc::ipv4::{self, Mask, Subnet};
use pwmp_client::pwmp_msg::settings::NodeSettings;
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

impl<T: RtcObject> RtcObject for Option<T> {
    fn checksum(&self) -> u32 {
        self.as_ref()
            .map_or_else(|| hash::crc32(&[0]), RtcObject::checksum)
    }

    fn new_empty() -> Self {
        None
    }
}
