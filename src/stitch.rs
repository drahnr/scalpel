use super::hex_convert::convert_hex2bin;
use byte_offset::*;
use bytes::BytesMut;
use errors::*;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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

#[derive(Debug, Deserialize, PartialEq)]
pub enum FileFormat {
    Bin,
    Hex,
    Elf,
    NoEnd,
}

impl Default for FileFormat {
    fn default() -> Self {
        FileFormat::Bin
    }
}

pub fn stitch_files(
    files: Vec<PathBuf>,
    offsets: Vec<ByteOffset>,
    output: PathBuf,
    fill_pattern: FillPattern,
    file_format: FileFormat,
) -> Result<()> {
    let offsets: Vec<usize> = offsets.iter().map(|ele| ele.as_usize()).collect();

    let (files, offsets) = sort_vec_by_offset(files, offsets)?;

    let stitched: Result<BytesMut> =
        files
            .iter()
            .zip(offsets.iter())
            .try_fold(BytesMut::new(), |stitched, (elem, offset)| {
                // before reading, check file ending
                let content = match check_file_format(elem.as_ref())? {
                    FileFormat::Bin | FileFormat::NoEnd => read_file(elem.as_ref()),
                    FileFormat::Hex => convert_hex2bin(elem.as_ref()),
                    _ => Err(ScalpelError::UnknownFileFormat
                        .context(format!("unimplemented extension {:?}", elem))
                        .into()),
                };

                Ok(
                    stitch(stitched, content?, offset, &fill_pattern).map_err(|e| {
                        ScalpelError::OverlapError
                            .context(format!("Failed to stitch {:?}: {}", elem, e))
                    })?,
                )
            });

    match file_format {
        FileFormat::Bin => write_file(Path::new(&output), stitched?)?,
        FileFormat::Hex => ::hex_convert::write_hex_file(Path::new(&output), stitched?)?,
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", file_format))
                .into())
        }
    }

    Ok(())
}

pub fn read_file(name: &Path) -> Result<BytesMut> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(name)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, name)))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    Ok(BytesMut::from(buf))
}

pub fn stitch(
    mut bytes: BytesMut,
    new: BytesMut,
    offset: &usize,
    fill_pattern: &FillPattern,
) -> Result<BytesMut> {
    if bytes.len() > *offset {
        return Err(ScalpelError::OverlapError
            .context(format!(
                "Offset {} is smaller than length {} of previous binaries",
                offset,
                bytes.len()
            ))
            .into());
    } else {
        match fill_pattern {
            FillPattern::Zero => bytes.resize(*offset, 0x0),
            FillPattern::One => bytes.resize(*offset, 0xFF),
            FillPattern::Random => {
                let mut padding = vec![0; *offset - bytes.len()];
                ::rand::thread_rng().try_fill(&mut padding[..])?;
                bytes.extend_from_slice(&padding);
            }
        }
        bytes.extend_from_slice(&new);
        debug!("Length: {}", &bytes.len());
        Ok(bytes)
    }
}

pub fn write_file(path: &Path, bytes: BytesMut) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, path)))?;

    file.write(&bytes)?;

    Ok(())
}

pub fn sort_vec_by_offset<T>(vec: Vec<T>, offset: Vec<usize>) -> Result<(Vec<T>, Vec<usize>)>
where
    T: Clone,
{
    let mut offset_sorted = offset.clone();
    offset_sorted.sort_unstable();

    let sorted_vec = offset_sorted
        .iter()
        .map(|elem| {
            let ind_o: usize = offset
                .iter()
                .position(|&s| &s == elem)
                .expect("Failed to sort");
            vec[ind_o].clone()
        })
        .collect();

    Ok((sorted_vec, offset_sorted))
}

pub fn check_file_format(name: &Path) -> Result<FileFormat> {
    let ext = match name.extension() {
        Some(e) => e.to_str().unwrap(),
        None => return Ok(FileFormat::NoEnd), // a bit risky to map all None to NoEnd, could also be a dir
    };

    match ext {
        "hex" => Ok(FileFormat::Hex),
        "bin" => Ok(FileFormat::Bin),
        "elf" => Ok(FileFormat::Elf),
        _ => Err(ScalpelError::UnknownFileFormat
            .context(format!("unimplemented extension {:?}", ext))
            .into()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stitch_it_bin() {
        let files = vec![
            PathBuf::from("tmp/test_bytes"),
            PathBuf::from("tmp/test_bytes"),
        ];
        let stitched = PathBuf::from("tmp/stitched_test");

        let offsets = vec![
            ByteOffset::new(0, Magnitude::Unit),
            ByteOffset::new(2, Magnitude::Ki),
        ];
        super::stitch_files(files, offsets, stitched, FillPattern::Zero, FileFormat::Bin)
            .expect("Failed to stitch two files");
        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("tmp/stitched_test")
                .map_err(|err| ScalpelError::OpeningError.context(err))
                .expect("Failed to open stitched file");

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .expect("Failed to read stitched file");
            buf
        };
        assert_eq!(buf.len(), 4096);
    }

    #[test]
    fn stitch_it_hex() {
        let files = vec![
            PathBuf::from("tmp/test_bytes"),
            PathBuf::from("tmp/test_bytes"),
        ];
        let stitched = PathBuf::from("tmp/stitched_test.hex");

        let offsets = vec![
            ByteOffset::new(0, Magnitude::Unit),
            ByteOffset::new(2, Magnitude::Ki),
        ];
        super::stitch_files(files, offsets, stitched, FillPattern::Zero, FileFormat::Hex)
            .expect("Failed to stitch two files");
        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("tmp/stitched_test.hex")
                .map_err(|err| ScalpelError::OpeningError.context(err))
                .expect("Failed to open stitched file");

            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("Failed to read stitched file");
            buf
        };
        // 16 bytes per row, 44 char per row + one EOF record
        let no_char = 4096 / 16 * (1 + 2 + 4 + 2 + 32 + 2 + 1) + 11;
        assert_eq!(buf.len(), no_char as usize);
    }

    #[test]
    fn try_stitch_elf() {
        let files = vec![
            PathBuf::from("tmp/test_bytes"),
            PathBuf::from("tmp/test_bytes"),
        ];
        let stitched = PathBuf::from("tmp/stitched_test.elf");

        let offsets = vec![
            ByteOffset::new(0, Magnitude::Unit),
            ByteOffset::new(2, Magnitude::Ki),
        ];
        let res = super::stitch_files(files, offsets, stitched, FillPattern::Zero, FileFormat::Elf);

        assert!(res.is_err());
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
        let ext = check_file_format(name.as_ref()).expect("Failed to check file format");

        assert_eq!(ext, FileFormat::NoEnd);
    }

}
