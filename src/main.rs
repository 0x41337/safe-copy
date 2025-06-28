mod core;

use std::path::Path;

use anyhow::Result;

use clap::Parser;

/// safe-copy: An atomic way to copy files safely and quickly.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to source file
    #[arg(short, long)]
    src: String,

    /// path to destination file
    #[arg(short, long)]
    dst: String,

    /// Optional block size in bytes
    #[arg(short = 'b', long)]
    block_size: Option<usize>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Checking if file exists
    let path = Path::new(&args.src);
    if !path.exists() || !path.is_file() {
        anyhow::bail!("File not found: {}", args.src);
    }

    // Sets the block size
    let block_size = args.block_size.unwrap_or(core::constants::BLOCK_SIZE);

    // Start copying
    core::copier::copy_file(&args.src, &args.dst, block_size)?;

    Ok(())
}
