pub use failure::Error;
pub use failure::Fail;

use std;
pub type Result<X> = std::result::Result<X, Error>;

#[derive(Debug, Fail)]
pub enum ScalpelError {
    #[fail(display = "Failed to open.")]
    OpeningError,

    #[fail(display = "Failed to read.")]
    ReadingError,

    #[fail(display = "Failed to write.")]
    WritingError,

    #[fail(display = "Failed to resolve Path")]
    PathError,

    #[fail(display = "Failed to seek from start")]
    SeekError,

    #[fail(display = "Failed to parse Keys from .pk8")]
    ParsePk8Error,
    
    #[fail(display = "There is no key in this Signer yet")]
    KeyInitError,

    #[fail(display = "Content of file is not as expected")]
    ContentError,

    #[fail(display = "Wrong usage of arguments")]
    ArgumentError,

    #[fail(display = "Parsing Yada failed: {}", r)]
    ParsingError {r: String},

    #[fail(display = "Failed to stitch due to overlapping")]
    OverlapError,
}