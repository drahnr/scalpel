use crate::byte_offset::*;
use crate::intelhex::{convert_hex2bin, write_bin_as_hex_to_file};
use bytes::BytesMut;
use log::warn;
use rand::Rng;
use serde_derive::Deserialize;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::vec::Vec;
use tree_magic;

use failure::{format_err, Error};

pub type Result<X> = std::result::Result<X, Error>;

#[derive(Deserialize, Debug)]
pub enum FillPattern {
    Random,
    Zero,
    One,
}

impl Default for FillPattern {
    fn default() -> Self {
        FillPattern::Zero
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum MetaInfo {
    IntelHex,
    Bin,
}

impl Default for MetaInfo {
    fn default() -> Self {
        MetaInfo::Bin
    }
}

impl MetaInfo {
    #[allow(dead_code)]
    pub fn from_header_bytes(first_bytes: &[u8]) -> Result<MetaInfo> {
        match tree_magic::from_u8(first_bytes).as_str() {
            "binary" => Ok(MetaInfo::Bin),
            "ascii/text" => Ok(MetaInfo::IntelHex), // TODO actually attempt to parse maybe?
            _ => Err(format_err!("Unsupported error type")),
        }
    }

    pub fn from_content(path: &Path) -> Result<MetaInfo> {
        match tree_magic::from_filepath(path).as_str() {
            "application/octet-stream" => Ok(MetaInfo::Bin),
            "ascii/text" => Ok(MetaInfo::IntelHex),
            _ => Err(format_err!("Unspupported File Type")),
        }
    }

    pub fn from_file_extension(path: &Path) -> Result<MetaInfo> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("bin") => Ok(MetaInfo::Bin),
            Some("hex") => Ok(MetaInfo::IntelHex),
            Some(ext) => Err(format_err!("Unsupported file extension {}", ext)),
            None => Err(format_err!("File does not have an extension to guess")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotatedBytes {
    // TODO: reconsider name, they're not really annotated anymore?
    pub bytes: BytesMut,
}

impl AnnotatedBytes {
    pub fn new() -> Self {
        AnnotatedBytes {
            bytes: BytesMut::new(),
        }
    }

    pub fn save(self, path: &Path, meta_out: MetaInfo) -> Result<()> {
        match meta_out {
            MetaInfo::Bin => {
                let mut file = OpenOptions::new()
                    .truncate(true)
                    .write(true)
                    .create(true)
                    .open(path)?;

                file.write_all(&self.bytes)?;
            }
            MetaInfo::IntelHex => {
                write_bin_as_hex_to_file(path, self.bytes)?;
            }
        }

        Ok(())
    }

    pub fn load(path: &Path, meta_in: MetaInfo) -> Result<Self> {
        match meta_in {
            MetaInfo::Bin => {
                let mut file = OpenOptions::new().read(true).open(path)?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;

                Ok(AnnotatedBytes {
                    bytes: BytesMut::from(&bytes[..]),
                })
            }
            MetaInfo::IntelHex => Ok(AnnotatedBytes {
                bytes: convert_hex2bin(path)?,
            }),
        }
    }

    pub fn stance(&mut self, start: ByteOffset, size: ByteOffset) {
        if start.as_usize() > 0usize && start.as_usize() < self.bytes.len() {
            // split file in part before and after start index
            self.bytes = self.bytes.split_off(start.as_usize() - 1);
        } else {
            warn!("start {} is outside file size {}", start, self.bytes.len());
        }

        if size.as_usize() < self.bytes.len() {
            // split off everything after size
            self.bytes.truncate(size.as_usize());
        }
    }

    pub fn stitch(
        mut files: Vec<(AnnotatedBytes, ByteOffset)>,
        fill_pattern: FillPattern,
    ) -> Result<AnnotatedBytes> {
        files.sort_by(|a, b| a.1.cmp(&b.1));

        files
            .into_iter()
            .try_fold(AnnotatedBytes::new(), |mut stitched, (elem, offset)| {
                // check if offset is greater than length
                if stitched.bytes.len() > offset.as_usize() {
                    return Err(format_err!(
                        "Offset {} smaller than current file {}",
                        offset,
                        stitched.bytes.len()
                    ));
                }
                match fill_pattern {
                    FillPattern::Zero => stitched.bytes.resize(offset.as_usize(), 0x00),
                    FillPattern::One => stitched.bytes.resize(offset.as_usize(), 0xFF),
                    FillPattern::Random => {
                        let mut padding = vec![0; offset.as_usize() - stitched.bytes.len()];
                        ::rand::thread_rng().try_fill(&mut padding[..])?;
                        stitched.bytes.extend_from_slice(&padding);
                    }
                }
                stitched.bytes.extend_from_slice(&elem.bytes);
                Ok(stitched)
            })
    }

    pub fn graft(
        &mut self,
        replace: AnnotatedBytes,
        start: ByteOffset,
        size: ByteOffset,
        fill_pattern: FillPattern,
    ) -> Result<()> {
        // [prefix replacement postfix]

        let mut output = self.bytes.clone();
        // split file in part before and after start index
        let after = output.split_off(start.as_usize());

        // get length of replacing part
        let prefix_len = &output.len();

        if replace.bytes.len() > size.as_usize() {
            return Err(format_err!(
                "Failed to graft bytes, size is smaller than replacing bytes"
            ));
        }
        // append replacing bytes
        output.extend_from_slice(&replace.bytes);

        // fill missing bytes
        match fill_pattern {
            FillPattern::Zero => output.resize(prefix_len + size.as_usize(), 0x0),
            FillPattern::One => output.resize(prefix_len + size.as_usize(), 0xFF),
            FillPattern::Random => {
                let mut padding = vec![0; size.as_usize() - replace.bytes.len()];
                ::rand::thread_rng().try_fill(&mut padding[..])?;
                output.extend_from_slice(&padding);
            }
        }

        // append the end
        output.extend_from_slice(&after[size.as_usize()..]);

        self.bytes = output;

        Ok(())
    }
}

// struct TestDataGraft {
//     idx: usize,
//     datasets: Vec<()>,
// }

// impl TestDataGraft {
//     pub fn new() -> Self {
//         Self {
//             idx: 0,
//             datasets: vec![(), (), ()],
//         }
//     }
// }

// impl Iterator for X {
//     type Item = (input, expected_output);
//     fn next() -> Option<Self::Item> {}
// }

// #[test]
// fn graft_everything() {
//     for item in X::new() {
//         item.graft();
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn graft() {
        let size = 80usize;

        let mut in_bytes = AnnotatedBytes::new();
        let mut graft_bytes = AnnotatedBytes::new();
        in_bytes.bytes.resize(200, 1u8);
        graft_bytes.bytes.resize(50, 2u8);
        let graft_len = graft_bytes.bytes.len();

        in_bytes
            .graft(
                graft_bytes,
                ByteOffset::new(10, Magnitude::Unit),
                ByteOffset::new(size as u64, Magnitude::Unit),
                FillPattern::One,
            )
            .expect("Failed to graft");

        let ones = vec![1u8; 200 - 10 - size];
        let twos = vec![2u8; graft_len];
        let ffs = vec![0xffu8; size - graft_len];
        assert_eq!(in_bytes.bytes[0..10], ones[..10]);
        assert_eq!(in_bytes.bytes[10..10 + graft_len], twos[..]);
        assert_eq!(in_bytes.bytes[10 + graft_len..10 + size], ffs[..]);
        assert_eq!(in_bytes.bytes[10 + size..], ones[..]);
    }

    #[test]
    fn stitch() {
        let bos: Vec<ByteOffset> = vec![
            ByteOffset::new(0, Magnitude::Unit),
            ByteOffset::new(1, Magnitude::K),
            ByteOffset::new(1, Magnitude::Ki),
        ];

        let mut byts: Vec<AnnotatedBytes> = vec![
            AnnotatedBytes::new(),
            AnnotatedBytes::new(),
            AnnotatedBytes::new(),
        ];

        byts[0].bytes.resize(30, 1u8);
        byts[1].bytes.resize(4, 2u8);
        byts[2].bytes.resize(100, 3u8);

        let stitch_vec: Vec<(AnnotatedBytes, ByteOffset)> =
            byts.into_iter().zip(bos.into_iter()).collect();

        let stitched =
            AnnotatedBytes::stitch(stitch_vec, FillPattern::One).expect("Failed to stitch");

        let ones = vec![1u8; 100];
        let twos = vec![2u8; 4];
        let threes = vec![3u8; 100];
        let ffs = vec![255u8; 1000];
        assert_eq!(stitched.bytes[..30], ones[..30]);
        assert_eq!(stitched.bytes[30..1000], ffs[30..1000]);
        assert_eq!(stitched.bytes[1000..1004], twos[..]);
        assert_eq!(stitched.bytes[1004..1024], ffs[..20]);
        assert_eq!(stitched.bytes[1024..], threes[..]);
    }

    #[test]
    fn stance() {
        let mut in_bytes = AnnotatedBytes::new();
        in_bytes.bytes.resize(100, 2u8);

        let start = ByteOffset::new(10, Magnitude::Unit);
        let size = ByteOffset::new(40, Magnitude::Unit);
        in_bytes.stance(start.clone(), size.clone());
        assert_eq!(in_bytes.bytes.len(), size.as_usize());

        in_bytes.bytes.resize(100, 2u8);
        let size = ByteOffset::new(1, Magnitude::K);
        in_bytes.stance(start.clone(), size);
        assert_eq!(in_bytes.bytes.len(), 101 - start.as_usize());
    }
}
