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
use std::path::PathBuf;

mod byte_offset;
// mod cut;
mod errors;
mod intelhex;

mod refactored;
use refactored::{AnnotatedBytes, FillPattern, MetaInfo};
// mod replace;
// mod stitch;

use byte_offset::*;
use errors::*;

const USAGE: &'static str = "
scalpel

Usage:
  scalpel stance [--start=<start>] --end=<end> --output=<output> <input> [--file-format=<format>]
  scalpel stance [--start=<start>] --size=<size> --output=<output> <input> [--file-format=<format>]
  scalpel stitch (--files=<files> --offset=<offset>)... --output=<output> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel graft [--start=<start>] --end=<end> --replace=<replace> --output=<output> <input> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel graft [--start=<start>] --size=<size> --replace=<replace> --output=<output> <input> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel (-h | --help)
  scalpel (-v |--version)

Commands:
  stance     extract bytes from a binary file
  stitch  stitchs binaries together, each file starts at <offset> with (random|one|zero) padding, accepted file formats: binary, IntelHex
  graft   replace a section with <replace> specfied by start and end/size

Options:
  -h --help                     Show this screen.
  -v --version                  Show version.
  --start=<start>               Start byte offset of the section to stance/graft. If omitted, set to 0.
  --end=<end>                   The end byte offset which will not be included.
  --size=<size>                 Alternate way to sepcify the <end> combined with start.
  --fill-pattern=<fill_patern>  Specify padding style for stitching files (random|one|zero)
  --replace=<replace>           File which replaces the original part
  --file-format=<format>        define output file format as either bin (default) or hex, has no influence on file ending!
";

// TODO clean up stale struct member variables
#[derive(Debug, Deserialize)]
struct Args {
    cmd_stance: bool,
    cmd_stitch: bool,
    cmd_graft: bool,
    arg_input: PathBuf,
    flag_start: Option<ByteOffset>,
    flag_end: Option<ByteOffset>,
    flag_size: Option<ByteOffset>,
    flag_files: Vec<PathBuf>,
    flag_offset: Vec<ByteOffset>,
    flag_output: PathBuf,
    flag_fill_pattern: Option<FillPattern>,
    flag_file_format: Option<MetaInfo>,
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
        let start = args.flag_start.unwrap_or(Default::default()); // if none, set to 0
        let size: ByteOffset = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(format_err!(
                    "Either end or size has to be specified, not both"
                ));
            }
            if start >= end {
                return Err(format_err!("Start must not be greater than end"));
            }
            end - start
        } else if let Some(size) = args.flag_size {
            size
        } else {
            return Err(format_err!("Either end addr or size has to be specified"));
        };
        // let fragment_size = args.flag_fragment.unwrap_or(Default::default()).as_u64(); // CHUNK 8192 from cut

        // guess meta_in from file
        let meta_in = unimplemented!();

        let mut in_bytes = AnnotatedBytes::load(&args.arg_input, meta_in)?;

        in_bytes.stance(start, size)?;

        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_stitch {
        // command stitch binaries together

        // guess from files
        let meta_in: MetaInfo = MetaInfo::Bin;

        // construct vec <(AnnoBytes, offsets)>
        let stitch_vec: Vec<(AnnotatedBytes, ByteOffset)> = args
            .flag_files
            .into_iter()
            .map(|f| {
                let meta_in: MetaInfo = unimplemented!();
                // TODO: get rid of unwrap
                AnnotatedBytes::load(&f, meta_in).unwrap()
            })
            .zip(args.flag_offset.into_iter())
            .collect();

        let out_bytes =
            AnnotatedBytes::stitch(stitch_vec, args.flag_fill_pattern.unwrap_or_default())?;

        //  impl default for Metainfo
        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        out_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_graft {
        // do input handling
        let start = args.flag_start.unwrap_or(Default::default()); // if none, set to 0
        let size: ByteOffset = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(format_err!("Either end or size has to be specified, not both"));
            }
            if start >= end {
                return Err(format_err!(
                        "end addr {1} should be larger than start addr {0}",
                        start, end
                    ));
            }
            end - start
        } else if let Some(size) = args.flag_size {
            size
        } else {
            return Err(format_err!("Either end addr or size has to be specified"));
        };

        let meta_in = unimplemented!();

        let mut in_bytes = AnnotatedBytes::load(&args.arg_input, meta_in)?;
        let graft_bytes = AnnotatedBytes::load(&args.flag_replace, meta_in)?;

        in_bytes.graft(
            graft_bytes,
            start,
            size,
            args.flag_fill_pattern.unwrap_or_default(),
        )?;

        //  impl default for Metainfo
        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else {
        Err(format_err!("No idea what you were thinking.."))
    }
}

quick_main!(run);
