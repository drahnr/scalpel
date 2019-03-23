pub use failure::Error;
pub use failure::Fail;

use std;
pub type Result<X> = std::result::Result<X, Error>;

#[derive(Debug, Fail, PartialEq, Eq)]
pub enum ScalpelError {
    #[fail(display = "Failed to open.")]
    OpeningError,

    #[fail(display = "Failed to read.")]
    ReadingError,

    #[fail(display = "Failed to write.")]
    WritingError,

    #[fail(display = "Failed to resolve Path")]
    PathError,

    #[fail(display = "Content of file is not as expected")]
    ContentError,

    #[fail(display = "Wrong usage of arguments")]
    ArgumentError,

    #[fail(display = "Parsing Yada failed: {}", r)]
    ParsingError { r: String },

    #[fail(display = "Failed to stitch due to overlapping")]
    OverlapError,

    #[fail(display = "Failed replace a section")]
    ReplaceError,

    #[fail(display = "Failed to convert hex record to binary")]
    HexError,

    #[fail(display = "Unknown file extension")]
    UnknownFileFormat,
}
