use std::{fmt, net::Ipv4Addr};
use std::net::SocketAddrV4;
use serde::{Deserialize, Serialize, Serializer};
use serde::de::{self, Visitor};

#[derive(Debug, Clone, Deserialize)]
pub struct Peers(pub Vec<SocketAddrV4>);
struct PeersVisitor;

impl<'de> Visitor<'de> for PeersVisitor {
    type Value = Peers;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a collection of Peers as a string")
    }

fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
where
E: de::Error, {
        
    if v.len() % 6 != 0 {
        return 
        Err(E::custom(format!("vector len must be a multiple of 6: {} does not fit the criteria", v.len())));
    } else {
        Ok (
            Peers (v.chunks_exact(6)
            .map(|slice_6|
            {
                SocketAddrV4::new(
                Ipv4Addr::new(slice_6[0], slice_6[1], slice_6[2], slice_6[3]),
                u16::from_be_bytes([slice_6[4], slice_6[5]])
                )
            }
        ).collect()
        ))
        }
    }
}

impl Serialize for Peers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        let mut single_slice = Vec::with_capacity(6 * self.0.len()); 
        for peer in &self.0 {
            single_slice.extend(peer.ip().octets());
            single_slice.extend(peer.port().to_be_bytes());
        }
        serializer.serialize_bytes(&single_slice)
    }
}

pub fn url_encode(t: &[u8; 20]) -> String{
        let mut encoded = String::with_capacity(3 * t.len());
        for &byte in t {
            encoded.push('%');
            encoded.push_str(&hex::encode(&[byte]));
        }
        encoded
    }