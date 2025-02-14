use std::ffi::OsString;
use std::fs::File;

use clap::Parser;
use zip_merge::ZipMerger;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(short, long, value_name = "output", verbatim_doc_comment)]
    output: OsString,
    
    files: Vec<OsString>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let mut merger = ZipMerger::new();
    for file_name in &args.files {
        let file = File::open(file_name)?;
        merger.add(file)?;
    }

    let file = File::create(args.output)?;
    merger.write(file)?;

    Ok(())
}
