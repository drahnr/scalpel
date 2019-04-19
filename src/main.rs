#[macro_use]
extern crate lazy_static;
extern crate regex;

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;
extern crate bytes;
extern crate docopt;
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

mod ops;
use crate::ops::{AnnotatedBytes, FillPattern, MetaInfo, Result};

use crate::byte_offset::*;
use crate::range::*;

const USAGE: &'static str = "
scalpel

Usage:
  scalpel stance --range=<range> --output=<output> <input> [--file-format=<format>]
  scalpel stitch (--input=<input> --offset=<offset>)... [--fill-pattern=<fill_pattern>] [--file-format=<format>] --output=<output>
  scalpel graft --replace=<replace> --range=<range>  [--fill-pattern=<fill_pattern>] [--file-format=<format>] --output=<output> <input>
  scalpel (-h | --help)
  scalpel (-v |--version)

Commands:
  stance  extract bytes from a binary file
  stitch  stitchs binaries together, each file starts at <offset> with (random|one|zero) padding, accepted file formats: binary, IntelHex
  graft   replace a section with <replace> specfied by start and end/size

Options:
  -h --help                     Show this screen.
  -v --version                  Show version.
  --range=<range>               byte range in rust slice-like sytnax: <start>..<end> yields [start,end) or <start>+<size> yields [start, start+size]
                                accepts the units K, Ki, M, Mi, G, Gi. Examples: 12K..4Ki   12M+512
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
    flag_input: Vec<PathBuf>,
    flag_offset: Vec<ByteOffset>,
    flag_range: Option<Range>,
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
        let range = args
            .flag_range
            .ok_or_else(|| format_err!("Missing range for stance"))?;

        // guess meta_in from file
        let path = args.arg_input;
        let meta_in = MetaInfo::from_file_extension(&path)
            .or_else::<Error, _>(|_err: Error| MetaInfo::from_content(&path))?;

        // load the input file
        let mut in_bytes = AnnotatedBytes::load(&path, meta_in)?;

        // do the cutting
        in_bytes.stance(range.start, range.size);

        // save output file
        let meta_out = args.flag_file_format.unwrap_or(meta_in);
        in_bytes.save(&args.flag_output, meta_out)?;

        Ok(())
    } else if args.cmd_stitch {
        // command stitch binaries together

        // construct vec <AnnotatedBytes>
        let stitch_vec = args.flag_input.into_iter().try_fold(
            // Vec::<AnnotatedBytes>::with_capacity(10),
            Vec::<AnnotatedBytes>::new(),
            |mut collection, path| {
                let meta_in: MetaInfo = MetaInfo::from_file_extension(&path)
                    .or_else::<Error, _>(|_err: Error| MetaInfo::from_content(&path))?;
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
        // command graft

        let range = args
            .flag_range
            .ok_or_else(|| format_err!("Missing range for graft"))?;

        // guess meta_in from files
        let path_in = args.arg_input;
        let path_graft = args.flag_replace;
        let meta_in: MetaInfo = MetaInfo::from_file_extension(&path_in)
            .or_else::<Error, _>(|_err: Error| MetaInfo::from_content(&path_in))?;
        let meta_graft: MetaInfo = MetaInfo::from_file_extension(&path_graft)
            .or_else::<Error, _>(|_err: Error| MetaInfo::from_content(&path_graft))?;

        // open input files
        let mut in_bytes = AnnotatedBytes::load(&path_in, meta_in)?;
        let graft_bytes = AnnotatedBytes::load(&path_graft, meta_graft)?;

        // put graft_bytes into in_bytes
        in_bytes.graft(
            graft_bytes,
            range.start,
            range.size,
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

#[cfg(test)]
mod test {
    use super::*;
    use docopt::Docopt;

    #[test]
    fn docopt_hex() {
        let argv = || {
            vec![
                "scalpel",
                "stance",
                "--range",
                "0x0..0x450",
                "--output",
                "a",
                "in",
            ]
        };
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.argv(argv().into_iter()).deserialize())
            .unwrap_or_else(|e| e.exit());

        assert!(args.cmd_stance);
        assert_eq!(
            args.flag_range,
            Some(Range::new(
                ByteOffset::new(0, Magnitude::Unit),
                ByteOffset::new(1104, Magnitude::Unit)
            ))
        );
    }

    #[test]
    fn docopt_dec() {
        let argv = || {
            vec![
                "scalpel",
                "stance",
                "--range",
                "20Ki..21Ki",
                "--output",
                "a",
                "in",
            ]
        };
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.argv(argv().into_iter()).deserialize())
            .unwrap_or_else(|e| e.exit());

        assert!(args.cmd_stance);
        assert_eq!(
            args.flag_range,
            Some(Range::new(
                ByteOffset::new(20, Magnitude::Ki),
                ByteOffset::new(1024, Magnitude::Unit)
            ))
        );
    }

    #[test]
    fn docopt_bad_hex() {
        let argv = || {
            vec![
                "scalpel",
                "stance",
                "--range",
                "0x20Fg..0xFA",
                "--output",
                "a",
                "in",
            ]
        };
        let args: std::result::Result<Args, docopt::Error> =
            Docopt::new(USAGE).and_then(|d| d.argv(argv().into_iter()).deserialize());
        assert!(args.is_err());
    }

    #[test]
    fn docopt_bigger_end() {
        let argv = || {
            vec![
                "scalpel",
                "stance",
                "--range",
                "0xFF..0xFE",
                "--output",
                "a",
                "in",
            ]
        };
        let args: std::result::Result<Args, docopt::Error> =
            Docopt::new(USAGE).and_then(|d| d.argv(argv().into_iter()).deserialize());
        assert!(args.is_err());
    }

    #[test]
    fn docopt_hex_and_dec() {
        let argv = || {
            vec![
                "scalpel", "stance", "--range", "0xFF..1K", "--output", "a", "in",
            ]
        };
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.argv(argv().into_iter()).deserialize())
            .unwrap_or_else(|e| e.exit());

        assert!(args.cmd_stance);
        assert_eq!(
            args.flag_range,
            Some(Range::new(
                ByteOffset::new(255, Magnitude::Unit),
                ByteOffset::new(745, Magnitude::Unit)
            ))
        );
    }

    #[test]
    fn docopt_byteoffset() {
        let argv = || {
            vec![
                "scalpel",
                "stitch",
                "--input",
                "bytes",
                "--offset",
                "10",
                "--input",
                "bytes",
                "--offset",
                "0x100",
                "--input",
                "bytes",
                "--offset",
                "1Ki",
                "--output",
                "stance.bin",
            ]
        };
        let args: Args = Docopt::new(USAGE)
            .and_then(|d| d.argv(argv().into_iter()).deserialize())
            .unwrap_or_else(|e| e.exit());

        let mut offset_it = args.flag_offset.iter();
        assert_eq!(
            offset_it.next().unwrap(),
            &ByteOffset::new(10, Magnitude::Unit)
        );
        assert_eq!(
            offset_it.next().unwrap(),
            &ByteOffset::new(256, Magnitude::Unit)
        );
        assert_eq!(
            offset_it.next().unwrap(),
            &ByteOffset::new(1, Magnitude::Ki)
        );
    }

}
