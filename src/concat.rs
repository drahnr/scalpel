use ring::signature;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use bytes::Bytes;

use errors::*;

/// open output file, add "-signed" to name
pub fn derive_output_filename(path: &Path) -> Result<String> {
    // get file
    let filename = path.to_str()
        .ok_or::<Error>(ScalpelError::PathError.into())?;

    let file_split: Vec<&str> = filename.rsplitn(2, '.').collect();

    Ok(if file_split.len() > 1 {
        format!("{}-signed.{}", file_split[1], file_split[0])
    } else {
        format!("{}-signed", file_split[0])
    })
}

/// takes a file and creates a copy with signature appended
pub fn append_signature(path: &Path, sig: &signature::Signature) -> Result<()> {
    let file_sig = derive_output_filename(path)?;

    // create output file
    let mut f_out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(Path::new(&file_sig))
        .map_err(|err| ScalpelError::OpeningError.context(err))?;

    // open input file
    let mut f_in = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| ScalpelError::OpeningError.context(err))?;

    // read input file to buffer
    let mut content: Vec<u8> = Vec::new();
    f_in.read_to_end(&mut content)
        .map_err(|err| ScalpelError::ReadingError.context(err))?;

    // write input to new file, afterwards append signature
    f_out
        .write_all(&content)
        .map_err(|err| ScalpelError::WritingError.context(err))?;

    let byte_sig = Bytes::from(sig.as_ref());

    f_out
        .write_all(&byte_sig)
        .map_err(|err| ScalpelError::WritingError.context(err))?;

    Ok(())
}

#[cfg(test)]
mod test {
    extern crate rand;
    use super::*;
    use self::rand::Rng;
    use std::iter;
    use signer::*;
    use std::io::{Seek, SeekFrom};

    #[test]
    fn test_append_signature() {
        let signer = Signer::random();

        //random content generation
        let mut rng = rand::thread_rng();
        let byte_victim = iter::repeat(1)
            .take(1000)
            .map(|_| rng.gen_range(1, 255))
            .collect::<Bytes>();
        let signature = signer.calculate_signature(&byte_victim)
            .expect("Failed signature from bytes");
        let path_victim = Path::new("tmp/test_bytes");
        append_signature(&path_victim, &signature).expect("Appending signature failed.");

        // open signed file and compare signature, path is hardcoded
        let path_victim = Path::new("tmp/test_bytes-signed");
        let mut f_in = OpenOptions::new()
            .read(true)
            .open(&path_victim)
            .expect("Failed to read signed File");
        // read from end of file for the length of singature
        let ref_sig = signature.as_ref();
        let mut read_sig = vec![0; ref_sig.len()];

        f_in.seek(SeekFrom::End(-(ref_sig.len() as i64)))
            .expect("Failed to seek from end");
        f_in.read(&mut read_sig).expect("Failed to Read Signature");
        println!("{}", read_sig.len());

        assert_eq!(ref_sig[..], read_sig[..]);
    }

    
}
