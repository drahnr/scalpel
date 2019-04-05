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
mod range;

mod refactored;
use crate::refactored::{AnnotatedBytes, FillPattern, MetaInfo, Result};

use crate::byte_offset::*;
use crate::range::*;

const USAGE: &'static str = "
scalpel

Usage:
  scalpel stance --range=<range> --output=<output> <input> [--file-format=<format>]
  scalpel stitch (--files=<files> --offset=<offset>)... --output=<output> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel graft  --replace=<replace> --range=<range> --output=<output> <input> [--fill-pattern=<fill_pattern>] [--file-format=<format>]
  scalpel (-h | --help)
  scalpel (-v |--version)

Commands:
  stance  extract bytes from a binary file
  stitch  stitchs binaries together, each file starts at <offset> with (random|one|zero) padding, accepted file formats: binary, IntelHex
  graft   replace a section with <replace> specfied by start and end/size

Options:
  -h --help                     Show this screen.
  -v --version                  Show version.
  --range=<range>               byte range in rust slice-like sytnax: <start>..<end> yields [start,end), accepts the units K, Ki, M, Mi, G, Gi. Example: 12K..4Ki
  --fill-pattern=<fill_patern>  Specify padding style for stitching files (random|one|zero)
  --replace=<replace>           File which replaces the original part
  --file-format=<format>        define output file format as either bin (default) or hex, has no influence on file ending!
";

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
    flag_range: Range,
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

        // // do input handling
        // let start = args.flag_start.unwrap_or_default(); // if none, set to 0
        // let size: ByteOffset = if let Some(end) = args.flag_end {
        //     if let Some(_) = args.flag_size {
        //         return Err(format_err!(
        //             "Either end or size has to be specified, not both"
        //         ));
        //     }
        //     if start >= end {
        //         return Err(format_err!("Start must not be greater than end"));
        //     }
        //     end - start.clone()
        // } else if let Some(size) = args.flag_size {
        //     size
        // } else {
        //     return Err(format_err!("Either end addr or size has to be specified"));
        // };
        let start = args.flag_range.start;
        let size = args.flag_range.end.clone() - start.clone();
        if size < ByteOffset::new(0, Magnitude::Unit) {
            return Err(format_err!(
                "End {} has to be greater than start {}",
                start,
                args.flag_range.end
            ));
        }

        // guess meta_in from file
        let path = args.arg_input;
        let meta_in = MetaInfo::from_file_extension(&path).or_else::<Error, _>(|_err: Error| {
            // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
            // Ok(mi)
            MetaInfo::from_content_alt(&path)
        })?;

        // load the input file
        let mut in_bytes = AnnotatedBytes::load(&path, meta_in)?;

        // do the cutting
        in_bytes.stance(start, size);

        // save output file
        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_stitch {
        // command stitch binaries together

        // construct vec <AnnotatedBytes>
        let stitch_vec = args.flag_files.into_iter().try_fold(
            // Vec::<AnnotatedBytes>::with_capacity(10),
            Vec::<AnnotatedBytes>::new(),
            |mut collection, path| {
                let meta_in: MetaInfo =
                    MetaInfo::from_file_extension(&path).or_else::<Error, _>(|_err: Error| {
                        // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                        // Ok(mi)
                        MetaInfo::from_content_alt(&path)
                    })?;
                let bytes = AnnotatedBytes::load(&path, meta_in)?;
                collection.push(bytes);
                Ok::<_, Error>(collection)
            },
        )?;

        // construct vec <(AnnotatedBytes, ByteOffset)>
        let stitch_vec = stitch_vec
            .into_iter()
            .zip(args.flag_offset.into_iter())
            .collect();

        // do the stitching
        let out_bytes =
            AnnotatedBytes::stitch(stitch_vec, args.flag_fill_pattern.unwrap_or_default())?;

        // save stitched output file
        // for consistent behaviour, should we also use the first meta_in as meta_out?
        let meta_out = args.flag_file_format.unwrap_or_default();
        out_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_graft {
        // // do input handling
        // let start = args.flag_start.unwrap_or_default(); // if none, set to 0
        // let size: ByteOffset = if let Some(end) = args.flag_end {
        //     if let Some(_) = args.flag_size {
        //         return Err(format_err!(
        //             "Either end or size has to be specified, not both"
        //         ));
        //     }
        //     if start >= end {
        //         return Err(format_err!(
        //             "end addr {1} should be larger than start addr {0}",
        //             start,
        //             end
        //         ));
        //     }
        //     end - start.clone()
        // } else if let Some(size) = args.flag_size {
        //     size
        // } else {
        //     return Err(format_err!("Either end addr or size has to be specified"));
        // };

        let start = args.flag_range.start;
        let size = args.flag_range.end.clone() - start.clone();
        if size < ByteOffset::new(0, Magnitude::Unit) {
            return Err(format_err!(
                "End {} has to be greater than start {}",
                start,
                args.flag_range.end
            ));
        }

        // guess meta_in from files
        let path_in = args.arg_input;
        let path_graft = args.flag_replace;
        let meta_in: MetaInfo =
            MetaInfo::from_file_extension(&path_in).or_else::<Error, _>(|_err: Error| {
                // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                // Ok(mi)
                MetaInfo::from_content_alt(&path_in)
            })?;
        let meta_graft: MetaInfo =
            MetaInfo::from_file_extension(&path_graft).or_else::<Error, _>(|_err: Error| {
                // let mi: MetaInfo = MetaInfo::from_content(&[0, 0, 0, 0, 0, 0])?;
                // Ok(mi)
                MetaInfo::from_content_alt(&path_graft)
            })?;

        // open input files
        let mut in_bytes = AnnotatedBytes::load(&path_in, meta_in)?;
        let graft_bytes = AnnotatedBytes::load(&path_graft, meta_graft)?;

        // put graft_bytes into in_bytes
        in_bytes.graft(
            graft_bytes,
            start,
            size,
            args.flag_fill_pattern.unwrap_or_default(),
        )?;

        // save output file
        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else {
        Err(format_err!("No idea what you were thinking.."))
    }
}

quick_main!(run);
