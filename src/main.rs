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

extern crate tree_magic;

use docopt::Docopt;
use std::path::PathBuf;

use failure::Error;

mod byte_offset;
mod intelhex;

mod refactored;
use refactored::{AnnotatedBytes, FillPattern, MetaInfo, Result};

use crate::byte_offset::*;

use std::borrow::Borrow;

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
        let start = args.flag_start.unwrap_or_default(); // if none, set to 0
        let size: ByteOffset = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(format_err!(
                    "Either end or size has to be specified, not both"
                ));
            }
            if start >= end {
                return Err(format_err!("Start must not be greater than end"));
            }
            end - start.clone()
        } else if let Some(size) = args.flag_size {
            size
        } else {
            return Err(format_err!("Either end addr or size has to be specified"));
        };
        // let fragment_size = args.flag_fragment.unwrap_or_default().as_u64(); // CHUNK 8192 from cut

        let path = args.arg_input;
        // guess meta_in from file
        let meta_in: MetaInfo =
                    MetaInfo::from_file_extension(&path).or_else::<Error, _>(|_err: Error| {
                        // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                        let mi: MetaInfo = MetaInfo::from_content_alt(&path)?;
                        Ok(mi)
                    })?;

        let mut in_bytes = AnnotatedBytes::load(&path, meta_in)?;

        // FIXME: derive clone for ByteOffset and do start.clone()?
        in_bytes.stance(start, size)?;

        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_stitch {
        // command stitch binaries together

        // construct vec <(AnnoBytes, offsets)>
        let stitch_vec = args.flag_files.into_iter().try_fold(
            Vec::<AnnotatedBytes>::with_capacity(10),
            |mut collection, path| {
                let meta_in: MetaInfo =
                    MetaInfo::from_file_extension(&path).or_else::<Error, _>(|_err: Error| {
                        // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                        let mi: MetaInfo = MetaInfo::from_content_alt(&path)?;
                        Ok(mi)
                    })?;

                let bytes = AnnotatedBytes::load(&path, meta_in)?;
                collection.push(bytes);
                Ok::<_, Error>(collection)
            },
        )?;

        let stitch_vec = stitch_vec
            .into_iter()
            .zip(args.flag_offset.into_iter())
            .collect();

        let out_bytes =
            AnnotatedBytes::stitch(stitch_vec, args.flag_fill_pattern.unwrap_or_default())?;

        //  impl default for Metainfo
        let meta_out = args.flag_file_format.unwrap_or_default();
        out_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_graft {
        // do input handling
        let start = args.flag_start.unwrap_or_default(); // if none, set to 0
        let size: ByteOffset = if let Some(end) = args.flag_end {
            if let Some(_) = args.flag_size {
                return Err(format_err!(
                    "Either end or size has to be specified, not both"
                ));
            }
            if start >= end {
                return Err(format_err!(
                    "end addr {1} should be larger than start addr {0}",
                    start,
                    end
                ));
            }
            end - start.clone()
        } else if let Some(size) = args.flag_size {
            size
        } else {
            return Err(format_err!("Either end addr or size has to be specified"));
        };

        let path_in = args.arg_input;
        let path_graft = args.flag_replace;
        // guess meta_in from file
        let meta_in: MetaInfo =
                    MetaInfo::from_file_extension(&path_in).or_else::<Error, _>(|_err: Error| {
                        // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                        let mi: MetaInfo = MetaInfo::from_content_alt(&path_in)?;
                        Ok(mi)
                    })?;
        let meta_graft: MetaInfo = MetaInfo::from_file_extension(&path_graft).or_else::<Error, _>(|_err: Error| {
                        // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                        let mi: MetaInfo = MetaInfo::from_content_alt(&path_graft)?;
                        Ok(mi)
                    })?;

        let mut in_bytes = AnnotatedBytes::load(&path_in, meta_in)?;
        let graft_bytes = AnnotatedBytes::load(&path_graft, meta_graft)?;

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
