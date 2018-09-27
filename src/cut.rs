use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

use errors::*;

pub fn cut_out_bytes(
    victim: String,
    output: String,
    start: u64,
    size: u64,
    fragment_size: usize,
) -> Result<()> {
    
    let mut f_in = OpenOptions::new()
        .read(true)
        .open(victim.as_str())
        .map_err(|err| ScalpelError::OpeningError.context(err).context(format!("Failed to open {} in R mode", victim)))?;

    let mut f_out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(output.as_str())
        .map_err(|err| ScalpelError::OpeningError.context(err).context(format!("Failed to open {} in W mode", output)))?;

    f_in.seek(SeekFrom::Start(start))
        .map_err(|err| ScalpelError::SeekError.context(err))?;

    let mut remaining = size;
    loop {
        let mut fragment = vec![0; fragment_size];
        f_in.read(&mut fragment[..])
            .map_err(|err| ScalpelError::ReadingError.context(err))?;

        if remaining < fragment_size as u64 {
            f_out
                .write_all(&fragment[0..(remaining as usize)])
                .map_err(|err| ScalpelError::WritingError.context(err))?;

            return Ok(());
        } else {
            f_out
                .write_all(&fragment[..])
                .map_err(|err| ScalpelError::WritingError.context(err))?;
            remaining -= fragment_size as u64;
        }
    }
}


#[cfg(test)]
mod test {
    extern crate rand;
    use super::*;

    #[test]
    fn test_cut_out() {
        // generate file with known byte content and cut some bytes out,
        // compare resulting file with bytes
        let bytes: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16];
        // write file with this content
        let victim = String::from("tmp/test_cut");
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
        let output = String::from("tmp/test_cut_out");
        cut_out_bytes(victim, output.clone(), 5, 4, 1).expect("Failed to cut");

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
        assert_eq!(output_bytes, &bytes[5..9]);
    }

}
