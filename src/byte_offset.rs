use failure::Error;
use regex::{Captures, Regex};
use serde::de;
use std::fmt;
use std::str::FromStr;

use crate::ops::Result;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Magnitude {
    Unit,
    K,
    Ki,
    M,
    Mi,
    G,
    Gi,
}

impl std::cmp::PartialOrd for Magnitude {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Magnitude {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Magnitude::Unit, Magnitude::Unit) => std::cmp::Ordering::Equal,
            (Magnitude::Unit, _) => std::cmp::Ordering::Less,

            (Magnitude::K, Magnitude::Unit) => std::cmp::Ordering::Greater,
            (Magnitude::K, Magnitude::K) => std::cmp::Ordering::Equal,
            (Magnitude::K, _) => std::cmp::Ordering::Less,

            (Magnitude::Ki, Magnitude::Unit) => std::cmp::Ordering::Greater,
            (Magnitude::Ki, Magnitude::K) => std::cmp::Ordering::Greater,
            (Magnitude::Ki, Magnitude::Ki) => std::cmp::Ordering::Equal,
            (Magnitude::Ki, _) => std::cmp::Ordering::Less,

            (Magnitude::M, Magnitude::Unit) => std::cmp::Ordering::Greater,
            (Magnitude::M, Magnitude::K) => std::cmp::Ordering::Greater,
            (Magnitude::M, Magnitude::Ki) => std::cmp::Ordering::Greater,
            (Magnitude::M, Magnitude::M) => std::cmp::Ordering::Equal,
            (Magnitude::M, _) => std::cmp::Ordering::Less,

            (Magnitude::Mi, Magnitude::Unit) => std::cmp::Ordering::Greater,
            (Magnitude::Mi, Magnitude::K) => std::cmp::Ordering::Greater,
            (Magnitude::Mi, Magnitude::Ki) => std::cmp::Ordering::Greater,
            (Magnitude::Mi, Magnitude::M) => std::cmp::Ordering::Greater,
            (Magnitude::Mi, Magnitude::Mi) => std::cmp::Ordering::Equal,
            (Magnitude::Mi, _) => std::cmp::Ordering::Less,

            (Magnitude::G, Magnitude::Unit) => std::cmp::Ordering::Greater,
            (Magnitude::G, Magnitude::K) => std::cmp::Ordering::Greater,
            (Magnitude::G, Magnitude::Ki) => std::cmp::Ordering::Greater,
            (Magnitude::G, Magnitude::M) => std::cmp::Ordering::Greater,
            (Magnitude::G, Magnitude::Mi) => std::cmp::Ordering::Greater,
            (Magnitude::G, Magnitude::G) => std::cmp::Ordering::Equal,
            (Magnitude::G, _) => std::cmp::Ordering::Less,

            (Magnitude::Gi, Magnitude::Gi) => std::cmp::Ordering::Equal,
            (Magnitude::Gi, _) => std::cmp::Ordering::Greater,
        }
    }
}

impl Default for Magnitude {
    fn default() -> Self {
        Magnitude::Unit
    }
}

impl Magnitude {
    pub fn parse(mag_str: &str) -> Result<Self> {
        match mag_str {
            "" => Ok(Magnitude::Unit),
            "K" => Ok(Magnitude::K),
            "Ki" => Ok(Magnitude::Ki),
            "M" => Ok(Magnitude::M),
            "Mi" => Ok(Magnitude::Mi),
            "G" => Ok(Magnitude::G),
            "Gi" => Ok(Magnitude::Gi),
            _ => {
                debug!("No idea what to do with {} as magnitude ", mag_str);
                Err(format_err!("Unknown Magnitude {}", mag_str))
            }
        }
    }

    pub fn as_u64(&self) -> u64 {
        match self {
            Magnitude::Unit => 1u64,
            Magnitude::K => 1000u64,
            Magnitude::Ki => 1024u64,
            Magnitude::M => 1000u64 * 1000u64,
            Magnitude::Mi => 1024u64 * 1024u64,
            Magnitude::G => 1000u64 * 1000u64 * 1000u64,
            Magnitude::Gi => 1024u64 * 1024u64 * 1024u64,
        }
    }

    pub fn as_usize(&self) -> usize {
        self.as_u64() as usize
    }
}

impl fmt::Display for Magnitude {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable_mag = match *self {
            Magnitude::Unit => "",
            Magnitude::K => "KB",
            Magnitude::Ki => "KiB",
            Magnitude::M => "MB",
            Magnitude::Mi => "MiB",
            Magnitude::G => "GB",
            Magnitude::Gi => "GiB",
        };
        write!(f, "{}", printable_mag)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ByteOffset {
    num: u64,
    magnitude: Magnitude,
}

impl ByteOffset {
    pub fn new(num: u64, magnitude: Magnitude) -> Self {
        Self { num, magnitude }
    }
    pub fn as_u64(&self) -> u64 {
        self.magnitude.as_u64() * self.num
    }
    pub fn as_usize(&self) -> usize {
        self.magnitude.as_usize() * (self.num as usize)
    }
}

impl<'de> de::Deserialize<'de> for ByteOffset {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct ByteOffsetVisitor;

        impl<'de> de::Visitor<'de> for ByteOffsetVisitor {
            type Value = ByteOffset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expected a ByteOffset")
            }

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<ByteOffset, E>
            where
                E: de::Error,
            {
                ByteOffset::from_str(value).map_err(|e| E::custom(format!("{:?}", e)))
            }
        }
        deserializer.deserialize_str(ByteOffsetVisitor)
    }
}

impl FromStr for ByteOffset {
    type Err = Error;

    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        lazy_static! {
            static ref REGEX: Regex =
                Regex::new(r"^(?:(0[xX]){1}([A-Fa-f0-9]+))|(?:([0-9]+)([KMGTE]i?)?)$").unwrap();
        }

        let byte_offset = REGEX
            .captures(s)
            .ok_or_else(|| {
                Err::<Captures, Error>(format_err!("Failed to parse {} to ByteOffset", s))
            })
            .and_then(|captures| {
                if captures.len() == 5 {
                    let byte_offset: ByteOffset = match &captures.get(1) {
                        Some(_) => {
                            let num_str = &captures[2];
                            let num = u64::from_str_radix(num_str, 16).map_err(|e| {
                                Err::<Captures, Error>(format_err!(
                                    "Failed to parse u64 from hex {}",
                                    e
                                ))
                            })?;
                            ByteOffset::new(num, Magnitude::Unit)
                        }
                        None => {
                            let num_str = &captures[3];
                            let magnitude_str = &captures.get(4).map_or("", |m| m.as_str());
                            let num = num_str.parse::<u64>().map_err(|e| {
                                Err::<Captures, Error>(format_err!("Failed to parse u64 {}", e))
                            })?;
                            let magnitude = Magnitude::parse(magnitude_str).map_err(|e| {
                                Err::<Captures, Error>(format_err!(
                                    "Failed to parse magnitude {}",
                                    e
                                ))
                            })?;
                            ByteOffset::new(num, magnitude)
                        }
                    };
                    Ok(byte_offset)
                } else {
                    Ok(Default::default())
                }
            })
            .map_err(|e| format_err!("{:?}", e))?;
        Ok(byte_offset)
    }
}

impl std::ops::Sub for ByteOffset {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let num = self.as_u64() - rhs.as_u64();
        // output has always magnitude Unit, we'd need a from_u64()
        // to parse from u64 into suitable magnitude?
        ByteOffset {
            num,
            magnitude: Magnitude::Unit,
        }
    }
}

impl std::ops::Add for ByteOffset {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let num = self.as_u64() + rhs.as_u64();
        // output has always magnitude Unit, we'd need a from_u64()
        // to parse from u64 into suitable magnitude?
        ByteOffset {
            num,
            magnitude: Magnitude::Unit,
        }
    }
}

impl fmt::Display for ByteOffset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.num, self.magnitude)
    }
}

impl std::cmp::PartialOrd for ByteOffset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for ByteOffset {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_u64().cmp(&other.as_u64())
    }
}

// Old Stuff
// fn deserialize_suffix( n: &str) -> Result<u64>
// {
//     Ok(match n {
//         "Ki" => 1024,
//         "Mi" => 1024*1024,
//         "Gi" => 1024*1024*1024,
//         "K" => 1000,
//         "M" => 1000*1000,
//         "G" => 1000*1000*1000,
//         "" => 1,
//         n => return Err(
//                 ScalpelError::ArgumentError
//                 .context(format!("Bad Suffix: {}", n))
//                 .into(),
//             )
//     })
// }

// pub fn serialize_cmd_opt(flag: String) -> Result<u64> {

//     let suffix: u64 = deserialize_suffix(flag.trim_matches(char::is_numeric))?;
//     let val: u64 = flag.trim_matches(char::is_alphabetic).parse()?;

//     Ok(val * suffix)
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bo_decimal_from_string() {
        let byte_offset_string = vec!["0", "45K", "12Ki", "92M", "999Mi", "012G", "209Gi"];

        let byte_offsets: Vec<ByteOffset> = byte_offset_string
            .iter()
            .map(|bo| ByteOffset::from_str(bo).expect("Failed to parse"))
            .collect();

        assert_eq!(byte_offsets[0], ByteOffset::new(0, Magnitude::Unit));
        assert_eq!(byte_offsets[1], ByteOffset::new(45, Magnitude::K));
        assert_eq!(byte_offsets[2], ByteOffset::new(12, Magnitude::Ki));
        assert_eq!(byte_offsets[3], ByteOffset::new(92, Magnitude::M));
        assert_eq!(byte_offsets[4], ByteOffset::new(999, Magnitude::Mi));
        assert_eq!(byte_offsets[5], ByteOffset::new(12, Magnitude::G));
        assert_eq!(byte_offsets[6], ByteOffset::new(209, Magnitude::Gi));
    }

    #[test]
    fn bo_hex_from_string() {
        let byte_offset_strings = vec!["0x0", "0x100", "0XFAcBd"];
        let byte_offsets: Vec<ByteOffset> = byte_offset_strings
            .iter()
            .map(|bo| ByteOffset::from_str(bo).expect("failed to parse"))
            .collect();

        assert_eq!(byte_offsets[0], ByteOffset::new(0, Magnitude::Unit));
        assert_eq!(byte_offsets[1], ByteOffset::new(256, Magnitude::Unit));
        assert_eq!(byte_offsets[2], ByteOffset::new(1027261, Magnitude::Unit));
    }

    #[test]
    fn dec_bad_unit() {
        let bo_dec = vec!["1Ke", "10B", "100AA", "34Li"];
        bo_dec.iter().for_each(|bo_str| {
            let bo = ByteOffset::from_str(bo_str);
            assert!(bo.is_err());
        });
    }
}
