//! Command-line demo decoder for `aivdm`.
//!
//! Reads `!AIVDM`/`!AIVDO` sentences from a file (or stdin) and prints the
//! decoded message for each single-fragment line.

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;

use aivdm::{AisError, decode_line};
use anyhow::{Context, Result};
use clap::Parser;

mod print;

/// Decode AIS `!AIVDM`/`!AIVDO` sentences from a file or stdin.
#[derive(Parser)]
struct Args {
    /// Path to a file of NMEA lines; reads stdin if omitted.
    file: Option<PathBuf>,

    /// Print a summary line count instead of each decoded message.
    #[arg(long)]
    stats: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut decoded = 0u64;
    let mut skipped = 0u64;

    let lines = read_lines(args.file.as_deref())?;
    for line in lines {
        let line = line.context("reading input line")?;
        if line.trim().is_empty() {
            continue;
        }
        match decode_line(&line) {
            Ok(message) => {
                decoded += 1;
                if !args.stats {
                    print::print_message(&message);
                }
            }
            Err(AisError::IncompleteFragment) => {
                // multi-fragment reassembly is not wired up in this demo CLI
                skipped += 1;
            }
            Err(err) => {
                skipped += 1;
                eprintln!("skipping line ({err}): {line}");
            }
        }
    }

    if args.stats {
        println!("decoded: {decoded}, skipped: {skipped}");
    }

    Ok(())
}

fn read_lines(
    path: Option<&std::path::Path>,
) -> Result<Box<dyn Iterator<Item = io::Result<String>>>> {
    match path {
        Some(path) => {
            let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
            Ok(Box::new(BufReader::new(file).lines()))
        }
        None => Ok(Box::new(BufReader::new(io::stdin()).lines())),
    }
}
