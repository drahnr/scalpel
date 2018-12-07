use bytes::BytesMut;
use errors::*;
use ihex::reader::Reader;
use ihex::record::*;

use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::stitch::{stitch, FillPattern};

pub fn convert_hex2bin(file_name: &Path) -> Result<BytesMut> {
    let content = read_hex2string(file_name.as_ref())?;

    let mut ihex_reader = Reader::new_stopping_after_error_and_eof(content.as_str(), false, true);

    // use iterator
    ihex_reader.try_fold(BytesMut::new(), |bin, record| {
        hex_record2bin(record?, bin)
    })

}

fn hex_record2bin(record: Record, binary: BytesMut) -> Result<BytesMut> {

    let bin = match record {
        Record::Data { value, offset } => {
            stitch(binary, BytesMut::from(value), &(offset as usize), &FillPattern::Zero)?
        },
        Record::EndOfFile => binary,
        _ => {
            return Err(ScalpelError::HexError
                .context(format!("Unknown Record Type {:?}", record ))
                .into())
        }
    };

    Ok(bin)

}

fn read_hex2string(name: &Path) -> Result<String> {
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
        let mut string = read_hex2string(file.as_ref()).expect("Failed to read file");

        string.truncate(9);

        assert_eq!(string, String::from("[package]"));
    }

    #[test]
    fn test_read_string_err() {
        let file = PathBuf::from("NonExisitingFileName");

        let res = read_hex2string(file.as_ref());

        // is there a way to test for a specific error?
        // something with assert_eq!( res, ScalpelError::OpeneningError)
        assert!(res.is_err());
    }

    #[test]
    fn test_hex_convert() {
        let file = PathBuf::from("tmp/test.hex");

        let res = convert_hex2bin(&file);

        println!("{:?}", res);

        assert!(res.is_ok());
    }

    #[test]
    fn test_eof_record() {
        let record = Record::EndOfFile;
        let buf = BytesMut::from(vec!(0,0));
        let res = hex_record2bin(record, buf.clone());

        assert_eq!(buf, res.unwrap());
    }

    #[test]
    fn test_bad_record() {
        let record = Record::ExtendedLinearAddress(8);
        let buf = BytesMut::from(vec!(0,0));
        let res = hex_record2bin(record, buf.clone());

        // is there a way to test for a specific error?
        // something with assert_eq!( res, ScalpelError::HexError)
        assert!(res.is_err());
    }

}
