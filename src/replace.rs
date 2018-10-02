use bytes::{BytesMut};
use std::path::{Path, PathBuf};
use stitch::FillPattern;
use errors::*;
use rand::{Rng};
use std::fs::OpenOptions;
use std::io::Write;

pub fn replace_file(replace_path: PathBuf, input: PathBuf, output: String, start: u64, size: u64, fill_pattern: FillPattern) -> Result<()> {

    let content = ::stitch::read_file(input.as_ref())?;
    let replace_bytes = ::stitch::read_file(replace_path.as_ref())?;

    let replaced = replace(replace_bytes, content, start as usize, size as usize, fill_pattern)?;

    write_file(Path::new(&output), replaced)?;
    
    Ok(())
}


fn replace(replace: BytesMut, mut output: BytesMut, start: usize, size: usize, fill_pattern: FillPattern) -> Result<BytesMut> {

    if replace.len() > size {
        return Err(ScalpelError::ReplaceError.context(format!("Size {} of file larger than size {} of replacement section", replace.len(),size)).into());
    } 
    // split file in part before and after start index
    let after = output.split_off(start);

    let length = output.len();

    // append the replacement bytes
    output.extend_from_slice(&replace);

    // fill missing bytes
    match fill_pattern {
        FillPattern::Zero => output.resize(length+size, 0x0),
        FillPattern::One => output.resize(length+size, 0xFF),
        FillPattern::Random => {
            let mut padding = vec![0; size-replace.len()];
            ::rand::thread_rng().try_fill(&mut padding[..])?;
            output.extend_from_slice(&padding);
        },
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
        .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, path )))?;

    file.write(&bytes)?;

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
    use std::io::{Read};

    #[test]
    fn replace_a_bit() {
        let input =  PathBuf::from("tmp/test_bytes");
        let replacing = PathBuf::from("tmp/signme.bin");

        replace_file(replacing, input, "tmp/replaced".to_string(), 0, 630, FillPattern::One)
            .expect("Failed to replace file");

        let buf = {
            let mut file = OpenOptions::new()
                .read(true)
                .open("tmp/replaced")
                .map_err(|err| ScalpelError::OpeningError.context(err)).expect("Failed to open replaced file");

            let mut buf = Vec::new();
            file.read_to_end(&mut buf).expect("Failed to read replaced file");
            buf
        };

        assert_eq!(buf.len(), 2048);

        assert_eq!(buf[625..630], [0xFF; 5]);

    }



}