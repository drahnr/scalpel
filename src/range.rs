use failure::Error;
use regex::{Captures, Regex};
use serde::de;
use std::fmt;

use crate::byte_offset::ByteOffset;

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Range {
    pub start: ByteOffset,
    pub size: ByteOffset,
}

impl Range {
    pub fn new(start: ByteOffset, size: ByteOffset) -> Self {
        Range { start, size }
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
                lazy_static! {
                    static ref REGEX: Regex = Regex::new(
                        r"^((?:(0[xX]){1}([A-Fa-f0-9]+))|(?:[0-9]+([KMGTE]i?)?))(\.\.|\+)((?:(0[xX]){1}([A-Fa-f0-9]+))|(?:[0-9]+([KMGTE]i?)?))$"
                    )
                    .unwrap();
                }

                let range = REGEX
                    .captures(value)
                    .ok_or_else(|| {
                        Err::<Captures, Error>(format_err!("Failed to parse {} to Range", value))
                    })
                    .and_then(|captures| {
                        if captures.len() == 10 {
                            let start_str = &captures[1];
                            let size_or_end_str = &captures[6];
                            let start: ByteOffset =
                                start_str.parse::<ByteOffset>().map_err(|e| {
                                    Err::<Captures, Error>(format_err!(
                                        "Failed to parse start {}",
                                        e
                                    ))
                                })?;
                            let size: ByteOffset = match &captures[5] {
                                ".." => {
                                    let end =
                                        size_or_end_str.parse::<ByteOffset>().map_err(|e| {
                                            Err::<Captures, Error>(format_err!(
                                                "Failed to parse end {}",
                                                e
                                            ))
                                        })?;
                                    if &start > &end {
                                        return Err(Err(format_err!(
                                            "Start {} must be greater than end {}",
                                            &start,
                                            &end
                                        )));
                                    } else {
                                        end - start.clone()
                                    }
                                }
                                "+" => size_or_end_str.parse::<ByteOffset>().map_err(|e| {
                                    Err::<Captures, Error>(format_err!(
                                        "Failed to parse size {}",
                                        e
                                    ))
                                })?,
                                _ => {
                                    return Err(Err(format_err!(
                                        "Failed to parse {}",
                                        &captures[5]
                                    )));
                                }
                            };
                            Ok(Range::new(start, size))
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
