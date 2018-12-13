use bytes::BytesMut;
use errors::*;
use hex_convert::convert_hex2bin;
use rand::Rng;
use std::path::{Path, PathBuf};
use stitch::{check_file_format, read_file, write_file, FileFormat, FillPattern};

pub fn replace_file(
    replace_path: PathBuf,
    input: PathBuf,
    output: PathBuf,
    start: u64,
    size: u64,
    fill_pattern: FillPattern,
    file_format: FileFormat,
) -> Result<()> {
    let content = match check_file_format(input.as_ref())? {
        FileFormat::Bin | FileFormat::NoEnd => read_file(input.as_ref())?,
        FileFormat::Hex => convert_hex2bin(input.as_ref())?,
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", input))
                .into())
        }
    };
    let replace_bytes = match check_file_format(replace_path.as_ref())? {
        FileFormat::Bin | FileFormat::NoEnd => read_file(replace_path.as_ref())?,
        FileFormat::Hex => convert_hex2bin(replace_path.as_ref())?,
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", replace_path))
                .into())
        }
    };

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
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", file_format))
                .into())
        }
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

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::Read;

    #[test]
    fn replace_a_bit_bin() {
        let input = PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/signme.bin");
        let replaced = PathBuf::from("tmp/replaced");

        replace_file(
            replacing,
            input,
            replaced,
            0,
            630,
            FillPattern::One,
            FileFormat::Bin,
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
        let replaced = PathBuf::from("tmp/replaced.hex");

        replace_file(
            replacing,
            input,
            replaced,
            0,
            642,
            FillPattern::One,
            FileFormat::Hex,
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
        let no_char = 2048 / 16 * (1 + 2 + 4 + 2 + 32 + 2 + 1) + 11;
        assert_eq!(buf.len(), no_char);

        // line 39 contains the end of the replaced section with all 0xFF
        assert_eq!(
            buf.lines().nth(39).unwrap(),
            ":1002700095FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF8"
        );
        // line 40 contains the start of the original file with 0xFFFF at the beginning
        assert_eq!(
            buf.lines().nth(40).unwrap(),
            ":10028000FFFFCAD2D3D4D5D6D7D8D9DAE2E3E4E592"
        );
    }

    #[test]
    fn try_replace_elf() {
        let input = PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/signme.bin");
        let replaced = PathBuf::from("tmp/replaced.elf");

        let res = replace_file(
            replacing,
            input,
            replaced,
            0,
            630,
            FillPattern::One,
            FileFormat::Elf,
        );

        assert!(res.is_err());
    }

    #[test]
    fn replace_a_bit_from_hex() {
        let input = PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/test.hex");
        let replaced = PathBuf::from("tmp/replaced.hex");

        replace_file(
            replacing,
            input,
            replaced,
            0,
            630,
            FillPattern::One,
            FileFormat::Hex,
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
        let no_char = 2048 / 16 * (1 + 2 + 4 + 2 + 32 + 2 + 1) + 11;
        assert_eq!(buf.len(), no_char);

        // line 39 contains the start of the replaced section
        assert_eq!(
            buf.lines().nth(39).unwrap(),
            ":10027000FFFFFFFFFFFFB7B8B9BAC2C3C4C5C6C707"
        );
    }

}
