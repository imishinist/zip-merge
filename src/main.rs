use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use zip_merge::ZipMerger;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(short, long, value_name = "output", verbatim_doc_comment)]
    output: PathBuf,

    files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
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
