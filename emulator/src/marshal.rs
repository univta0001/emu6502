#[cfg(all(feature = "serde_support", feature = "flate"))]
use flate2::Compression;
#[cfg(all(feature = "serde_support", feature = "flate"))]
use flate2::read::GzDecoder;
#[cfg(all(feature = "serde_support", feature = "flate"))]
use flate2::write::GzEncoder;
#[cfg(feature = "serde_support")]
use serde::de::{Error, Unexpected};
#[cfg(all(feature = "serde_support", feature = "flate"))]
use serde::ser;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "serde_support")]
use std::collections::BTreeMap;
#[cfg(all(feature = "serde_support", feature = "flate"))]
use std::io::{Read, Write};

#[cfg(feature = "serde_support")]
pub fn hex_to_u8(c: u8) -> std::io::Result<u8> {
    match c {
        b'A'..=b'F' => Ok(c - b'A' + 10),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'0'..=b'9' => Ok(c - b'0'),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid hex char",
        )),
    }
}

#[cfg(all(feature = "serde_support", feature = "flate"))]
pub fn as_opt_hex<S: Serializer>(
    value: &Option<Vec<u8>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if let Some(ref v) = *value {
        return as_hex_6bytes(v, serializer);
    }
    serializer.serialize_none()
}

#[cfg(all(feature = "serde_support", feature = "flate"))]
fn as_hex_6bytes<S: Serializer>(v: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = BTreeMap::new();
    let mut addr = 0;
    let mut count = 0;
    let mut s = String::new();
    let mut gz_vector = Vec::new();
    {
        let mut gz = GzEncoder::new(&mut gz_vector, Compression::fast());
        let status = gz.write_all(v);
        if status.is_err() {
            return Err(ser::Error::custom("Unable to encode data"));
        }
    }

    for value in gz_vector {
        if count >= 0x40 {
            let addr_key = format!("{addr:06X}");
            map.insert(addr_key, s);
            s = String::new();
            count = 0;
            addr += 0x40;
        }
        let hex = format!("{value:02X}");
        s.push_str(&hex);
        count += 1;
    }

    if !s.is_empty() {
        let addr_key = format!("{addr:06X}");
        map.insert(addr_key, s);
    }
    BTreeMap::serialize(&map, serializer)
}

#[cfg(feature = "serde_support")]
pub fn as_hex<S: Serializer>(v: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    let mut map = BTreeMap::new();
    let mut addr = 0;
    let mut count = 0;
    let mut s = String::new();
    let format_addr_6bytes = v.len() >= 0x10000;
    for value in v {
        if count >= 0x40 {
            let addr_key = if format_addr_6bytes {
                format!("{addr:06X}")
            } else {
                format!("{addr:04X}")
            };
            map.insert(addr_key, s);
            s = String::new();
            count = 0;
            addr += 0x40;
        }
        let hex = format!("{value:02X}");
        s.push_str(&hex);
        count += 1;
    }

    if !s.is_empty() {
        let addr_key = if format_addr_6bytes {
            format!("{addr:06X}")
        } else {
            format!("{addr:04X}")
        };
        map.insert(addr_key, s);
    }
    BTreeMap::serialize(&map, serializer)
}

#[cfg(all(feature = "serde_support", feature = "flate"))]
pub fn from_hex_opt<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<u8>>, D::Error> {
    let map: Option<BTreeMap<String, String>> = Option::deserialize(deserializer)?;

    if let Some(map) = map {
        if map.is_empty() {
            return Ok(None);
        }

        let mut gz_vector = Vec::new();
        let mut addr = 0;
        for key in map.keys() {
            let addr_value_6bytes = format!("{addr:06X}");
            if *key != addr_value_6bytes {
                return Err(Error::invalid_value(
                    Unexpected::Seq,
                    &"Invalid key. Addr not in sequence",
                ));
            }

            let value = &map[key];
            if value.len() % 2 != 0 {
                return Err(Error::invalid_value(Unexpected::Seq, &"Invalid hex length"));
            }
            for pair in value.chars().collect::<Vec<_>>().chunks(2) {
                let result = (hex_to_u8(pair[0] as u8).map_err(Error::custom)? << 4)
                    | hex_to_u8(pair[1] as u8).map_err(Error::custom)?;
                gz_vector.push(result);
            }
            addr += 0x40;
        }

        let mut v: Vec<u8> = Vec::new();
        {
            let mut decoder = GzDecoder::new(&gz_vector[..]);
            let status = decoder.read_to_end(&mut v);
            if status.is_err() {
                return Err(Error::invalid_value(
                    Unexpected::Seq,
                    &"Unable to decode data",
                ));
            }
        }
        Ok(Some(v))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "serde_support")]
fn from_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let map = BTreeMap::<String, String>::deserialize(deserializer)?;
    let mut v = Vec::new();
    let mut addr = 0;
    for key in map.keys() {
        let addr_value_4bytes = format!("{addr:04X}");
        let addr_value_6bytes = format!("{addr:06X}");
        if *key != addr_value_4bytes && *key != addr_value_6bytes {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Invalid key. Addr not in sequence",
            ));
        }

        let value = &map[key];
        if value.len() % 2 != 0 {
            return Err(Error::invalid_value(Unexpected::Seq, &"Invalid hex length"));
        }
        for pair in value.chars().collect::<Vec<_>>().chunks(2) {
            let result = (hex_to_u8(pair[0] as u8).map_err(Error::custom)? << 4)
                | hex_to_u8(pair[1] as u8).map_err(Error::custom)?;
            v.push(result);
        }
        addr += 0x40;
    }
    Ok(v)
}

#[cfg(feature = "serde_support")]
pub fn from_hex_64k<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let result = from_hex(deserializer);
    if let Ok(ref value) = result {
        if value.len() != 0x10000 {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Array should be 64K",
            ));
        }
    }
    result
}

#[cfg(feature = "serde_support")]
pub fn from_hex_12k<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let result = from_hex(deserializer);
    if let Ok(ref value) = result {
        if value.len() != 0x3000 && value.len() % 0x3000 * 8 != 0 {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Array should be 12K or multiple of 128k",
            ));
        }
    }
    result
}

#[cfg(feature = "serde_support")]
pub fn from_hex_32k<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let result = from_hex(deserializer);
    if let Ok(ref value) = result {
        if value.len() != 0x8000 {
            return Err(Error::invalid_value(
                Unexpected::Seq,
                &"Array should be 32K",
            ));
        }
    }
    result
}
