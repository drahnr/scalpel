use bytes::{BytesMut};
use std::path::{Path, PathBuf};
use stitch::FillPattern;
use errors::*;
use rand::{Rng};

pub fn graft_file(graft_path: PathBuf, input: PathBuf, output: String, start: u64, size: u64, fill_pattern: FillPattern) -> Result<()> {

    let content = ::stitch::read_file(input.as_ref())?;
    let graft_bytes = ::stitch::read_file(graft_path.as_ref())?;

    let replaced = graft(graft_bytes, content, start as usize, size as usize, fill_pattern)?;

    ::stitch::write_file(Path::new(&output), replaced)?;
    
    Ok(())
}


fn graft(graft: BytesMut, mut output: BytesMut, start: usize, size: usize, fill_pattern: FillPattern) -> Result<BytesMut> {

    if graft.len() > size {
        return Err(ScalpelError::OverlapError.context(format!("Size {} of file larger than size {} of replacement section", graft.len(),size)).into());
    } 
    // split file in part before and after start index
    let after = output.split_off(start);
    // append the replacement bytes
    output.extend_from_slice(&graft);
    let length = output.len();
    // fill missing bytes
    match fill_pattern {
        FillPattern::Zero => output.resize(length+size, 0x0),
        FillPattern::One => output.resize(length+size, 0xFF),
        FillPattern::Random => {
            let mut padding = vec![0; size-graft.len()];
            ::rand::thread_rng().try_fill(&mut padding[..])?;
            output.extend_from_slice(&padding);
        },
    }
    // append the end
    output.extend_from_slice(&after[size..]);

    Ok(output)
}