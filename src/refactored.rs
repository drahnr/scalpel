use crate::byte_offset::*;
use crate::intelhex::{convert_hex2bin, write_bin_as_hex_to_file};
use bytes::BytesMut;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::vec::Vec;
use tree_magic;

use failure::Error;

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
    pub fn from_content(first_bytes: &[u8]) -> Result<MetaInfo> {
        match tree_magic::from_u8(first_bytes).as_str() {
            "binary" => Ok(MetaInfo::Bin),
            "ascii/text" => Ok(MetaInfo::IntelHex), // TODO actually attempt to parse maybe?
            _ => Err(format_err!("Unsupported error type")),
        }
    }
    /// TODO: Alternative impl of from_content, takes a path directly instead of reading the first bytes
    pub fn from_content_alt(path: &Path) -> Result<MetaInfo> {
        match tree_magic::from_filepath(path).as_str() {
            "binary" => Ok(MetaInfo::Bin),
            "Ascii/text" => Ok(MetaInfo::IntelHex),
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
                    bytes: BytesMut::from(bytes),
                })
            }
            MetaInfo::IntelHex => Ok(AnnotatedBytes {
                bytes: convert_hex2bin(path)?,
            }),
        }
    }

    pub fn stance(&mut self, start: ByteOffset, size: ByteOffset) {
        if start.as_usize() > 0usize {
            // split file in part before and after start index
            self.bytes = self.bytes.split_off(start.as_usize() - 1);
        }
        // split off everything after size
        self.bytes.split_off(size.as_usize());
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

        output.extend_from_slice(&replace.bytes);
        let curr_len = output.len();

        // TODO: check sizes?

        // fill missing bytes
        match fill_pattern {
            FillPattern::Zero => output.resize(curr_len + size.as_usize(), 0x0),
            FillPattern::One => output.resize(curr_len + size.as_usize(), 0xFF),
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
