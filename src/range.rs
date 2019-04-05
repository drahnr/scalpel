use failure::Error;
use regex::{Captures, Regex};
use serde::de;
use std::fmt;

use crate::byte_offset::ByteOffset;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Range {
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Range {
    pub fn new(start: ByteOffset, end: ByteOffset) -> Self {
        Range { start, end }
    }
}

impl<'de> de::Deserialize<'de> for Range {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct RangeVisitor;

        impl<'de> de::Visitor<'de> for RangeVisitor {
            type Value = Range;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expected Range")
            }

            fn visit_str<E>(self, value: &str) -> ::std::result::Result<Range, E>
            where
                E: de::Error,
            {
                lazy_static!{
                    static ref REGEX: Regex = Regex::new(r"^(([0-9]+)((?:[KMGTE]i?)?))\.\.(([0-9]+)((?:[KMGTE]i?)?))$").unwrap();
                }

                let range = REGEX
                    .captures(value)
                    .ok_or_else(|| Err::<Captures, Error>(format_err!("Failed to parse value")))
                    .and_then(|captures| {
                        if captures.len() == 7 {
                            let start_str = &captures[1];
                            let end_str = &captures[4];
                            let start: ByteOffset = start_str.parse::<ByteOffset>().map_err(|e| {
                                Err::<Captures, Error>(format_err!("Failed to parse start {}", e))
                            })?;
                            let end: ByteOffset = end_str.parse::<ByteOffset>().map_err(|e| {
                                Err::<Captures, Error>(format_err!("Failed to parse end {}", e))
                            })?;
                            Ok(Range::new(start, end))
                        } else {
                            Ok(Default::default())
                        }
                    })
                    .map_err(|e| E::custom(format!("{:?}", e)))?;
                Ok(range)
            }
        }
        deserializer.deserialize_str(RangeVisitor)
    }
}