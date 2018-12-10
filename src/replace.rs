use bytes::BytesMut;
use errors::*;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use stitch::{FillPattern, FileFormat};

pub fn replace_file(
    replace_path: PathBuf,
    input: PathBuf,
    output: String,
    start: u64,
    size: u64,
    fill_pattern: FillPattern,
    file_format: FileFormat
) -> Result<()> {
    let content = ::stitch::read_file(input.as_ref())?;
    let replace_bytes = ::stitch::read_file(replace_path.as_ref())?;

    let replaced = replace(
        replace_bytes,
        content,
        start as usize,
        size as usize,
        fill_pattern,
    )?;

    match file_format {
        FileFormat::Bin => write_file(Path::new(&output), replaced)?,
        FileFormat::Hex => ::hex_convert::write_hex_file(Path::new(&output), replaced)?,
        _ => return Err(ScalpelError::UnknownFileFormat
                        .context(format!("unimplemented extension {:?}", file_format))
                        .into()),
    }

    Ok(())
}

fn replace(
    replace: BytesMut,
    mut output: BytesMut,
    start: usize,
    size: usize,
    fill_pattern: FillPattern,
) -> Result<BytesMut> {
    if replace.len() > size {
        return Err(ScalpelError::ReplaceError
            .context(format!(
                "Size {} of file larger than size {} of replacement section",
                replace.len(),
                size
            ))
            .into());
    }
    // split file in part before and after start index
    let after = output.split_off(start);

    let length = output.len();

    // append the replacement bytes
    output.extend_from_slice(&replace);

    // fill missing bytes
    match fill_pattern {
        FillPattern::Zero => output.resize(length + size, 0x0),
        FillPattern::One => output.resize(length + size, 0xFF),
        FillPattern::Random => {
            let mut padding = vec![0; size - replace.len()];
            ::rand::thread_rng().try_fill(&mut padding[..])?;
            output.extend_from_slice(&padding);
        }
    }

    // append the end
    output.extend_from_slice(&after[size..]);

    Ok(output)
}

fn write_file(path: &Path, bytes: BytesMut) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, path)))?;

    file.write(&bytes)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Read;

    #[test]
    fn replace_a_bit_bin() {
        let input = PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/signme.bin");

        replace_file(
            replacing,
            input,
            "tmp/replaced".to_string(),
            0,
            630,
            FillPattern::One,
            FileFormat::Bin
        )
        .expect("Failed to replace file");

        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("tmp/replaced")
                .map_err(|err| ScalpelError::OpeningError.context(err))
                .expect("Failed to open replaced file");

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .expect("Failed to read replaced file");
            buf
        };

        assert_eq!(buf.len(), 2048);

        assert_eq!(buf[625..630], [0xFF; 5]);
    }

    #[test]
    fn replace_a_bit_hex() {
        let input = PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/signme.bin");

        replace_file(
            replacing,
            input,
            "tmp/replaced.hex".to_string(),
            0,
            630,
            FillPattern::One,
            FileFormat::Hex
        )
        .expect("Failed to replace file");

        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("tmp/replaced.hex")
                .map_err(|err| ScalpelError::OpeningError.context(err))
                .expect("Failed to open replaced file");

            let mut buf = String::new();
            file.read_to_string(&mut buf)
                .expect("Failed to read replaced file");
            buf
        };

                    // 16 bytes per row, 44 char per row + one EOF record
        let no_char = 2048 / 16 * (1+2+4+2+32+2+1) +11;
        assert_eq!(buf.len(), no_char);

        // assert_eq!(buf[625..630], [0xFF; 5]);
        // line 39 contains the start of the replaced section
        assert_eq!(buf.lines().nth(39).unwrap(), ":1002700095FFFFFFFFFFB7B8B9BAC2C3C4C5C6C771");
    }

}
