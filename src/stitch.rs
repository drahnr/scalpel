use std::fs::OpenOptions;
use bytes::{BytesMut};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use errors::*;
use rand::{Rng};
use byte_offset::*;
use std::ffi::OsStr;

#[derive(Deserialize, Debug)]
pub enum FillPattern { Random, Zero, One}

impl Default for FillPattern {
    fn default() -> Self {
        FillPattern::Zero
    }
}

#[derive(Debug, PartialEq)]
pub enum FileFormat { Bin, Hex, Elf, NoEnd }

pub fn stitch_files(files: Vec<PathBuf>, offsets: Vec<ByteOffset>, output: String, fill_pattern: FillPattern) -> Result<()> {
    
    let offsets: Vec<usize> = offsets.iter().map(|ele| ele.as_usize()).collect();

    let (files, offsets) = sort_vec_by_offset(files, offsets)?;

    let stitched: Result<BytesMut>
     = files.iter().zip(offsets.iter()).try_fold(BytesMut::new(), |stitched, (elem, offset)| {
        // before reading, check file ending
        let content = read_file(elem.as_ref())
            .map_err(|e| {
                return ScalpelError::OpeningError.context(e)
            })?;
        
        Ok(stitch(stitched, content, offset, &fill_pattern).map_err(|e| ScalpelError::OverlapError.context(format!("Failed to stitch {:?}: {}", elem, e)))?)
        
    });

    write_file(Path::new(&output), stitched?)?;

    Ok(())
}

pub fn read_file(name: &Path) -> Result<BytesMut> {

    let mut file = OpenOptions::new()
        .read(true)
        .open(name)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, name )))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    Ok(BytesMut::from(buf))
}

pub fn stitch(mut bytes: BytesMut, new: BytesMut, offset: &usize, fill_pattern: &FillPattern) -> Result<BytesMut> {
    if bytes.len() > *offset {
        return Err(ScalpelError::OverlapError.context(format!("Offset {} is smaller than length {} of previous binaries", offset, bytes.len())).into());
    } else {
        match fill_pattern {
            FillPattern::Zero => bytes.resize(*offset, 0x0),
            FillPattern::One => bytes.resize(*offset, 0xFF),
            FillPattern::Random => {
                let mut padding = vec![0; *offset-bytes.len()];
                ::rand::thread_rng().try_fill(&mut padding[..])?;
                bytes.extend_from_slice(&padding);
            },
        }
        bytes.extend_from_slice(&new);
        debug!("Length: {}", &bytes.len());
        Ok(bytes)
    }
}

fn write_file(path: &Path, bytes: BytesMut) -> Result<()> {
    
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, path )))?;

    file.write(&bytes)?;

    Ok(())
}

pub fn sort_vec_by_offset<T>(vec: Vec<T>, offset: Vec<usize>) -> Result<(Vec<T>, Vec<usize>)>
where T: Clone,
{

    let mut offset_sorted = offset.clone();
    offset_sorted.sort_unstable();

    let sorted_vec =  offset_sorted.iter().map(|elem|  {
        let ind_o: usize = offset.iter().position(|&s| &s == elem).expect("Failed to sort");
        vec[ind_o].clone()
    }).collect();

    Ok((sorted_vec, offset_sorted))
}

fn check_file_format(name: &Path) -> Result<FileFormat> {
    let ext = match name.extension() {
        Some(e) => e.to_str().unwrap(),
        None => return Err(ScalpelError::UnknownFileFormat.context(format!("No extension found") ).into()),
    };

    match ext {
        "hex" => Ok(FileFormat::Hex),
        "bin" => Ok(FileFormat::Bin),
        "elf" => Ok(FileFormat::Elf),
        _ => Err(ScalpelError::UnknownFileFormat.context(format!("unimplemented extension {:?}", ext)).into()),
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stitch_it() {
        let files = vec![ PathBuf::from("tmp/test_bytes"), PathBuf::from("tmp/test_bytes")];

        let offsets = vec![ByteOffset::new(0,Magnitude::Unit), ByteOffset::new(2,Magnitude::Ki)];
        super::stitch_files(files, offsets, "stitched_test".to_string(), FillPattern::Zero).expect("Failed to stitch two files");
        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("stitched_test")
                .map_err(|err| ScalpelError::OpeningError.context(err)).expect("Failed to open stitched file");

            let mut buf = Vec::new();
            file.read_to_end(&mut buf).expect("Failed to read stitched file");
            buf
        };
        assert_eq!(buf.len(), 4096);
    }

    #[test]
    fn test_ext_hex() {
        let name = PathBuf::from("tmp/test.hex");
        let ext = check_file_format(name.as_ref()).expect("Failed to check file format");

        assert_eq!(ext, FileFormat::Hex);
    }

    #[test]
    fn test_ext_bin() {
        let name = PathBuf::from("tmp/signme.bin");
        let ext = check_file_format(name.as_ref()).expect("Failed to check file format");

        assert_eq!(ext, FileFormat::Bin);
    }

    #[test]
    fn test_no_ext() {
        let name = PathBuf::from("tmp/test_bytes");
        let ext = check_file_format(name.as_ref());

        assert!(ext.is_err());
    }


}
