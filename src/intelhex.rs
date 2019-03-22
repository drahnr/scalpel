use bytes::BytesMut;
use errors::*;
use ihex::reader::Reader;
use ihex::record::*;
use ihex::writer;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;

pub fn convert_hex2bin(file_name: &Path) -> Result<BytesMut> {
    let content = read_hex2string(file_name.as_ref())?;

    let mut ihex_reader = Reader::new_stopping_after_error_and_eof(content.as_str(), false, true);

    // use iterator
    ihex_reader.try_fold(BytesMut::new(), |bin, record| hex_record2bin(record?, bin))
}

fn hex_record2bin(record: Record, binary: BytesMut) -> Result<BytesMut> {
    let bin = match record {
        Record::Data { value, offset } => {
            let mut bin = BytesMut::new();
            bin.resize(offset as usize, 0x00);
            bin.extend_from_slice(&value[..]);
            bin
        }
        Record::EndOfFile => binary,
        _ => {
            return Err(ScalpelError::HexError
                .context(format!("Unknown Record Type {:?}", record))
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
    let buf = buf.to_string();

    Ok(buf)
}

pub fn write_bin_as_hex_to_file(path: &Path, mut bytes: BytesMut) -> Result<()> {
    // let vec_content = bytes.to_vec();

    let byte_count = 16;
    let rec_count: f32 = bytes.len() as f32 / byte_count as f32;
    let mut records: Vec<Record> = Vec::new();

    for ind in 0..rec_count.ceil() as usize {
        if &bytes.len() > &byte_count {
            // according to doc: split_to() is exclusive on the right: +1
            // but tests state the opposite...
            let data = bytes.split_to(byte_count);
            records.push(Record::Data {
                offset: (byte_count * ind) as u16,
                value: data.to_vec(),
            });
        } else {
            records.push(Record::Data {
                offset: 16 * ind as u16,
                value: bytes.to_vec(),
            });
        }
    }

    let eof_rec = Record::EndOfFile;
    records.push(eof_rec);

    let ihex_obj = writer::create_object_file_representation(&records)?;

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, path)))?;

    write!(file, "{}", ihex_obj)?; // TODO write_all

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::BufMut;
    use std::path::PathBuf;

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
        let buf = BytesMut::from(vec![0, 0]);
        let res = hex_record2bin(record, buf.clone());

        assert_eq!(buf, res.unwrap());
    }

    #[test]
    fn test_bad_record() {
        let record = Record::ExtendedLinearAddress(8);
        let buf = BytesMut::from(vec![0, 0]);
        let res = hex_record2bin(record, buf.clone());

        // is there a way to test for a specific error?
        // something with assert_eq!( res, ScalpelError::HexError)
        assert!(res.is_err());
    }

    #[test]
    fn test_write_hex() {
        let name = PathBuf::from("tmp/test_write.hex");
        let mut bytes = BytesMut::with_capacity(255);

        bytes.put_u64_le(1);
        bytes.put_u64_le(2);
        bytes.put_u64_le(3);
        bytes.put_u64_le(4);
        bytes.put_u64_le(5);
        bytes.put_u64_le(6);
        bytes.put_u64_le(7);
        bytes.put_u64_le(8);
        bytes.put_u64_le(9);
        bytes.put_u64_le(10);
        bytes.put_u64_le(11);
        bytes.put_u64_le(254);

        write_bin_as_hex_to_file(name.as_ref(), bytes).expect("Failed to write bytes to hex file");

        let mut hex_file = OpenOptions::new()
            .read(true)
            .open("tmp/test_write.hex")
            .map_err(|err| ScalpelError::OpeningError.context(err))
            .expect("Failed to open stitched file");

        let mut content = String::new();
        hex_file
            .read_to_string(&mut content)
            .expect("Failed to read hex file");
        println!("{}", content);

        let hex = ":1000000001000000000000000200000000000000ED
:1000100003000000000000000400000000000000D9
:1000200005000000000000000600000000000000C5
:1000300007000000000000000800000000000000B1
:1000400009000000000000000A000000000000009D
:100050000B00000000000000FE0000000000000097
:00000001FF";

        // add a more sophisitcated test
        assert_eq!(content, hex);
    }

}
