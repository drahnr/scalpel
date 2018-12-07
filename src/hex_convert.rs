use bytes::BytesMut;
use errors::*;
use ihex::reader::Reader;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn convert_hex_to_bin(file_name: PathBuf) -> Result<()> {
    let content = read_hex_to_string(file_name.as_ref())?;

    let ihex_reader = Reader::new_stopping_after_error_and_eof(content.as_str(), false, true);

    for record in ihex_reader {
        println!("{:?}", record);
    }

    Ok(())
}

fn read_hex_to_string(name: &Path) -> Result<String> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(name)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, name)))?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    Ok(buf)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_string() {
        let file = PathBuf::from("Cargo.toml");
        let mut string = read_hex_to_string(file.as_ref()).expect("Failed to read file");

        string.truncate(9);

        assert_eq!(string, String::from("[package]"));
    }

    #[test]
    fn test_read_string_err() {
        let file = PathBuf::from("NonExisitingFileName");

        let res = read_hex_to_string(file.as_ref());

        // is there a way to test for a specific error?
        // something with assert_eq!( res, ScalpelError::OpeneningError)
        assert!(res.is_err());
    }

    #[test]
    fn test_hex_convert() {
        let file = PathBuf::from("tmp/test.hex");

        let res = convert_hex_to_bin(file);

        assert!(res.is_err());

    }

}
