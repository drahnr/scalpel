#[macro_use]
extern crate lazy_static;
extern crate regex;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate untrusted;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
extern crate docopt;
extern crate ring;
extern crate serde;
#[macro_use]
extern crate common_failures;
#[macro_use]
extern crate failure;
extern crate ihex;
extern crate rand;

use docopt::Docopt;
use std::path::{Path, PathBuf};

mod byte_offset;
// mod cut;
mod errors;
mod intelhex;

mod refactored;
use refactored::{AnnotatedBytes, FillPattern};
// mod replace;
// mod stitch;

use byte_offset::*;
use errors::*;

const USAGE: &'static str = "
scalpel

Usage:
  scalpel stance [--fragment=<fragment>] [--start=<start>] --end=<end> --output=<output> <file> [--file-format=<format>]
  scalpel stance [--fragment=<fragment>] [--start=<start>] --size=<size> --output=<output> <file> [--file-format=<format>]
  scalpel stitch (--binary=<binary> --offset=<offset>)... --output=<output> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel graft [--start=<start>] --end=<end> --replace=<replace> --output=<output> <input> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel graft [--start=<start>] --size=<size> --replace=<replace> --output=<output> <input> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel (-h | --help)
  scalpel (-v |--version)

Commands:
  cut     extract bytes from a binary file
  stitch  stitchs binaries together, each file starts at <offset> with (random|one|zero) padding, accepted file formats: binary, IntelHex
  graft   replace a section with <replace> specfied by start and end/size

Options:
  -h --help                     Show this screen.
  -v --version                  Show version.
  --start=<start>               Start byte offset of the section to cut out. If omitted, set to 0.
  --end=<end>                   The end byte offset which will not be included.
  --size=<size>                 Alternate way to sepcify the <end> combined with start.
  --fragment=<fragment>         Define the size of the fragment/chunk to read/write at once. [Default: 8192]
  --format=<format>             Specify the key format, eihter pkcs8, pem, bytes or new
  --fill-pattern=<fill_patern>  Specify padding style for stitching (random|one|zero)
  --replace=<replace>           File which replaces the original part
  --file-format=<format>        define output file format as either bin (default) or hex, has no influence on file ending!
";

// TODO clean up stale struct member variables
#[derive(Debug, Deserialize)]
struct Args {
    cmd_stance: bool,
    cmd_stitch: bool,
    cmd_graft: bool,
    flag_start: Option<ByteOffset>,
    flag_end: Option<ByteOffset>,
    flag_size: Option<ByteOffset>,
    flag_fragment: Option<ByteOffset>,
    flag_output: Option<PathBuf>,
    arg_file: PathBuf,
    arg_files: Vec<String>,
    arg_input: PathBuf,
    flag_offset: Vec<ByteOffset>,
    flag_fill_pattern: Option<FillPattern>,
    flag_format: Option<String>,
    flag_replace: PathBuf,
    flag_version: bool,
    flag_help: bool,
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const NAME: &'static str = env!("CARGO_PKG_NAME");

// TODO use the run from traits and combine with the cmd if else and error handling but get rid of ScalpelError (maybe?)
// TODO or use Err(...)? pattern instead
fn run() -> Result<()> {
    env_logger::init();

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    // check arguments
    if args.flag_version {
        println!("{} {}", NAME, VERSION);
        Ok(())
    } else if args.flag_help {
        println!("{}", USAGE);
        Ok(())
    } else if args.cmd_stance {
        // command stance

        // do input handling
        let start = args.flag_start.unwrap_or(Default::default()).as_u64(); // if none, set to 0
        let size: u64 = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(format_err!("Either end or size has to be specified, not both"));
            }
            let end = end.as_u64();
            if start >= end {
                return Err(format_err!("Start must not be greater than end"));
            }
            end - start
        } else if let Some(size) = args.flag_size {
            let size = size.as_u64();
            size
        } else {
            return Err(format_err!("Either end addr or size has to be specified"));
        };
        // let fragment_size = args.flag_fragment.unwrap_or(Default::default()).as_u64(); // CHUNK 8192 from cut

        let meta_out = unimplemented!();
        let in_bytes = AnnotatedBytes::load(args.arg_file, meta_out)?;

        in_bytes.stance(start, size).and_then(|_| {
            info!("Cutting success");
            Ok(())
        })
    } else if args.cmd_stitch {
        // command stitch binaries together

        stitch::stitch_files(
            args.flag_binary,
            args.flag_offset,
            args.flag_output.unwrap(),
            args.flag_fill_pattern.unwrap_or_default(),
            args.flag_file_format.unwrap_or_default(),
        )?;

        Ok(())
    } else if args.cmd_graft {
        // do input handling
        let start = args.flag_start.unwrap_or(Default::default()).as_u64(); // if none, set to 0
        let size: u64 = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(ScalpelError::ArgumentError
                    .context("Either end or size has to be specified, not both")
                    .into());
            }
            let end = end.as_u64();
            if start >= end {
                return Err(ScalpelError::ArgumentError
                    .context(format!(
                        "end addr {1} should be larger than start addr {0}",
                        start, end
                    ))
                    .into());
            }
            end - start
        } else if let Some(size) = args.flag_size {
            let size = size.as_u64();
            size
        } else {
            return Err(ScalpelError::ArgumentError
                .context("Either end addr or size has to be specified")
                .into());
        };

        replace::replace_file(
            args.flag_replace,
            args.arg_input,
            args.flag_output.unwrap(),
            start,
            size,
            args.flag_fill_pattern.unwrap_or_default(),
            args.flag_file_format.unwrap_or_default(),
        )?;

        Ok(())
    } else {
        Err(ScalpelError::ArgumentError
            .context("No idea what you were thinking..")
            .into())
    }
}

quick_main!(run);
