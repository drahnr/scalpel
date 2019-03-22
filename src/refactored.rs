use super::intelhex::{convert_hex2bin, write_bin_as_hex_to_file};
use byte_offset::*;
use bytes::BytesMut;
use errors::*;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::vec::Vec;

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

    pub fn load(path : &Path, meta_out : MetaInfo) -> Result<Self> {
        match meta_out {
            MetaInfo::Bin => {
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(path)?;
                let mut bytes = std::vec::new();
                file.read_to_end(&mut bytes);

                Ok(AnnotatedBytes {
                    bytes : BytesMut::from(bytes),
                })
                
            }
            MetaInfo::IntelHex => {
                Ok(AnnotatedBytes {
                    bytes : convert_hex2bin(path)?,
                })
            }
        }
    }
}


impl AnnotatedBytes {

    // pub fn stance(&mut self, start: ByteOffset, size : ByteOffset) -> Result<()> {
    // convertion ByteOffset -> u64 currently done in main, keep it that way?
    pub fn stance(&mut self, start: u64, size : u64) -> Result<()> {
        
        // split file in part before and after start index
        self.bytes = self.bytes.split_off(start as usize - 1);
        // split off everything after size
        self.bytes.split_off(size as usize);
        Ok(())
    }

    pub fn stitch(
        files: Vec<(AnnotatedBytes, usize)>,
        fill_pattern: FillPattern,
        meta_out : MetaInfo,
    ) -> Result<AnnotatedBytes> {

        files
            .iter()
            .try_fold(AnnotatedBytes::new(), |stitched, (elem, offset)| {
                // before reading, check file ending
                // let content = elem.convert_to(MetaInfo::Bin)?;

                match fill_pattern {
                    FillPattern::Zero => stitched.bytes.resize(*offset, 0x00),
                    FillPattern::One => stitched.bytes.resize(*offset, 0xFF),
                    FillPattern::Random => {
                        let mut padding = vec![0; *offset - stitched.bytes.len()];
                        ::rand::thread_rng().try_fill(&mut padding[..])?;
                        stitched.bytes.extend_from_slice(&padding);
                    }
                }
                stitched.bytes.extend_from_slice(&elem.bytes);
                Ok(stitched)
            })  
    }

    pub fn graft(&mut self, replace : AnnotatedBytes, start: ByteOffset, size : ByteOffset, fill_pattern : FillPattern) -> Result<()> {
        // [ prefix replacement postfix]

        // split file in part before and after start index
        let mut output = self.bytes.clone();
        let after = output.split_off(start);

        output.extend_from_slice(&replace.bytes);

        // fill missing bytes
        match fill_pattern {
            FillPattern::Zero => output.resize(before.len() + size, 0x0),
            FillPattern::One => output.resize(before.len() + size, 0xFF),
            FillPattern::Random => {
                let mut padding = vec![0; size - replace.bytes.len()];
                ::rand::thread_rng().try_fill(&mut padding[..])?;
                output.extend_from_slice(&padding);
            }
        }

        // append the end
        output.extend_from_slice(&after[size..]);

        self.bytes = output;

        Ok(())
    }
}


struct TestDataGraft {
    idx : usize,
    datasets : Vec<()>,
}

impl TestDataGraft {
    pub fn new() -> Self {
        Self {
            idx : 0,
            datasets : vec![
                (),
                (),
                (),
            ]
        }
    }
}

impl Iterator for X {
    type Item = (input, expected_output);
    fn next() -> Option<Self::Item> {

    }
}



#[test]
fn graft_everything() {
    for item in X::new() {
        item.graft();
    }
}



// fn run() -> Result<()> {

//     // read

//     let meta_out = unimplemented!();
//     let meta_in = unimplemented!();
    
//     let bytes_in = unimplemented!();

//     let mut work = AnnotatedBytes::load(args.path_in, meta_in);

//     match cmd {
//         "stance" => {
//             work.stance()?;
//         },
//         "graft" => {
//             work.graft()?;
//         },
//         "stitch" => {
//             work = AnnotatedBytes::stitch(args.files, args.fill_pattern)?;
//         },
//         "convert" => { 
//         },
//         _ => Err(format_err!("Noooope")),
//     }

//     work.save(args.path_out, meta_out)?;

//     Ok(())
// }



// quick_main!(run);
