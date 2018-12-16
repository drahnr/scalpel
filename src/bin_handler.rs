use std::path::{Path, PathBuf};
use bytes::BytesMut;

#[derive(Debug, Deserialize)]
pub struct BinHandler {
    file: Path,
    content: BytesMut,
}

impl BinHandler {

    pub fn new(file: PathBuf) -> Result<Self> {
        let content = read_file(file.as_ref())?;

        Ok(BinWrapper { file, content })
    }

    fn read_file(name: &Path) -> Result<BytesMut> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(name)
            .map_err(|err| ScalpelError::OpeningError.context(format!("{}: {:?}", err, name)))?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        Ok(BytesMut::from(buf))
    }
}
