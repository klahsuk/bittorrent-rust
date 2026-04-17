use serde::Serializer;
use serde::{Serialize, Deserialize, de::{self, Visitor}};
use std::{fmt};

struct HashesVisitor;

#[derive(Debug, Clone)]
pub struct Hashes(pub Vec<[u8; 20]>);


impl<'de> Visitor<'de> for HashesVisitor {
    type Value = Hashes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte string with length a multiple of 20")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !v.len().is_multiple_of(20) {
            return 
            Err(E::custom(format!("vector len must be a multiple of 20: {} does not fit the criteria", v.len())));
        }

        Ok(Hashes(
            v.chunks_exact(20)
            .map(|slice_20| slice_20.try_into().unwrap())
            .collect()
        ))
    }
}

impl <'de> Deserialize<'de> for Hashes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de> {
        deserializer.deserialize_bytes(HashesVisitor)
    }
}

impl Serialize for Hashes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let single_slice = self.0.concat();
        serializer.serialize_bytes(&single_slice)
    }
}
