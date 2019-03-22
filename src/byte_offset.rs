use errors::*;
use regex::{Captures, Regex};
use serde::de;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
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
                Err(ScalpelError::ParsingError {
                    r: format!("Unknown Magnitude {}", mag_str),
                }
                .into())
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

#[derive(Debug, Default)]
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
                lazy_static! {
                    static ref REGEX: Regex = Regex::new(r"^([0-9]+)((?:[KMGTE]i?)?)$").unwrap();
                }

                let byte_offset = REGEX
                    .captures(value)
                    .ok_or(Err::<Captures, ScalpelError>(ScalpelError::ParsingError {
                        r: "".to_string(),
                    }))
                    .and_then(|captures| {
                        if captures.len() == 3 {
                            let num_str = &captures[1];
                            let magnitude_str = &captures[2];
                            let num: u64 = num_str.parse::<u64>().map_err(|e| {
                                Err::<Captures, ScalpelError>(ScalpelError::ParsingError {
                                    r: format!("Failed to parse u64 {}", e),
                                })
                            })?;
                            let magnitude = Magnitude::parse(magnitude_str).map_err(|e| {
                                Err::<Captures, ScalpelError>(ScalpelError::ParsingError {
                                    r: format!("Failed to parse magnitude {}", e),
                                })
                            })?;
                            Ok(ByteOffset::new(num, magnitude))
                        } else {
                            Ok(Default::default())
                        }
                    })
                    .map_err(|e| E::custom(format!("{:?}", e)))?;
                Ok(byte_offset)
            }
        }
        deserializer.deserialize_str(ByteOffsetVisitor)
    }
}

impl std::ops::Sub for ByteOffset {
    type Output = Self;
    fn sub(self, RHS: Self) -> Self {
        let num = self.as_u64() - RHS.as_u64();
        // result has always magnitude Unit, we'd need a from_u64()
        // to parse from u64 into suitable magnitude?
        ByteOffset {
            num, magnitude: Magnitude::Unit,
        }   
    }
}

impl fmt::Display for ByteOffset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.num, self.magnitude)
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
