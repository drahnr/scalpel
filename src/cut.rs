use errors::*;
use hex_convert::{convert_hex2bin, write_hex_file};
use std::path::{Path, PathBuf};
use stitch::{check_file_format, read_file, write_file, FileFormat};

pub fn cut_out_bytes(
    victim: PathBuf,
    output: PathBuf,
    start: u64,
    size: u64,
    // fragment_size: usize,
    file_format: FileFormat,
) -> Result<()> {
    let mut content = match check_file_format(victim.as_ref())? {
        FileFormat::Bin | FileFormat::NoEnd => read_file(victim.as_ref())?,
        FileFormat::Hex => convert_hex2bin(victim.as_ref())?,
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", victim))
                .into())
        }
    };

    // split file in part before and after start index
    let mut out_buf = content.split_off(start as usize - 1);
    // split off everything after size
    out_buf.split_off(size as usize);

    match file_format {
        FileFormat::Bin => write_file(Path::new(&output), out_buf),
        FileFormat::Hex => write_hex_file(Path::new(&output), out_buf),
        _ => {
            return Err(ScalpelError::UnknownFileFormat
                .context(format!("unimplemented extension {:?}", file_format))
                .into())
        }
    }
}

#[cfg(test)]
mod test {
    extern crate rand;
    use super::*;
    use std::fs::OpenOptions;
    use std::io::{Read, Write};

    #[test]
    fn test_cut_out_bin() {
        // generate file with known byte content and cut some bytes out,
        // compare resulting file with bytes
        let bytes: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16];
        // write file with this content
        let victim = PathBuf::from("tmp/test_cut");
        {
            let mut file_tester = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(victim.clone())
                .expect("Failed to open file");
            file_tester
                .write_all(&bytes)
                .expect("Failed to write to file");
        }
        // cut bytes from this file
        let output = PathBuf::from("tmp/test_cut_out");
        cut_out_bytes(victim, output.clone(), 5, 4, FileFormat::Bin).expect("Failed to cut");

        // read content of output
        let mut output_bytes = vec![0, 0, 0, 0];
        let mut file_tested = OpenOptions::new()
            .read(true)
            .open(output)
            .expect("Failed to open ouput file");
        file_tested
            .read(&mut output_bytes)
            .expect("Failed to read file");

        println!("{:?}", output_bytes);
        assert_eq!(output_bytes, &bytes[4..8]);
    }

    #[test]
    fn test_cut_out_hex() {
        // generate file with known byte content and cut some bytes out,
        // compare resulting file with bytes
        let bytes: &[u8] = &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 27, 29,
        ];
        // write file with this content
        let victim = PathBuf::from("tmp/test_cut");
        {
            let mut file_tester = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(victim.clone())
                .expect("Failed to open file");
            file_tester
                .write_all(&bytes)
                .expect("Failed to write to file");
        }
        // cut bytes from this file
        let output = PathBuf::from("tmp/test_cut_out.hex");
        cut_out_bytes(victim, output.clone(), 5, 16, FileFormat::Hex).expect("Failed to cut");

        // read content of output
        let mut output_str = String::new();
        let mut file_tested = OpenOptions::new()
            .read(true)
            .open(output)
            .expect("Failed to open ouput file");
        file_tested
            .read_to_string(&mut output_str)
            .expect("Failed to read file");

        println!("{:?}", output_str);
        assert_eq!(
            output_str,
            ":1000000004050607080A0B0C0D0E0F10111213142D\n:00000001FF"
        );
    }
}
