use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;

use rayon::prelude::*;

use anyhow::{Ok, Result};
use cliclack::{intro, log, outro, progress_bar};
use sha2::{Digest, Sha256};

pub fn copy_file(src: &str, dst: &str, block_size: usize) -> Result<()> {
    // Create handle to get metadata
    let src_file = File::open(src)?;
    let metadata = src_file.metadata()?;
    let total_size = metadata.len();
    let total_blocks = (total_size + block_size as u64 - 1) / block_size as u64;

    intro("🎯 Starting the mission, copy the file (consider done.)")?;

    log::info(format!(
        "📦 Total size: {:.2} MB",
        total_size as f64 / 1024.0 / 1024.0
    ))?;
    log::info(format!("📏 Block size: {} bytes", block_size))?;
    log::info(format!("🧱 Total blocks: {}", total_blocks))?;

    // Check or create the destination file
    let dst_exists = std::path::Path::new(dst).exists();
    if dst_exists {
        let dst_metadata = File::open(dst)?.metadata()?;
        if dst_metadata.len() != total_size {
            log::warning("⚠️ Destination exists but with incorrect size. Truncating...")?;
            let dst_file = OpenOptions::new().write(true).open(dst)?;
            dst_file.set_len(total_size)?;
        } else {
            log::info("📄 Destination already exists with matching size. Checking blocks...")?;
        }
    } else {
        let dst_file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(dst)?;
        dst_file.set_len(total_size)?;
    }

    let progress = progress_bar(total_blocks);
    progress.start("📝 Copying file...");

    // Share paths between threads
    let src_path = Arc::new(src.to_string());
    let dst_path = Arc::new(dst.to_string());

    // Go through all blocks
    (0..total_blocks)
        .into_par_iter()
        .try_for_each(|block_index| {
            let offset = block_index * block_size as u64;

            // Clone file handles for independent thread usage
            let mut src_handle = File::open(&*src_path)?;
            let mut dst_handle = OpenOptions::new().read(true).write(true).open(&*dst_path)?;

            // Read
            let mut buffer = vec![0u8; block_size];
            src_handle.seek(SeekFrom::Start(offset))?;
            let read_bytes = src_handle.read(&mut buffer)?;
            buffer.truncate(read_bytes);

            let original_hash = Sha256::digest(&buffer);

            // Check destination before writing
            let mut existing_buf = vec![0u8; buffer.len()];
            dst_handle.seek(SeekFrom::Start(offset))?;
            dst_handle.read_exact(&mut existing_buf)?;
            let existing_hash = Sha256::digest(&existing_buf);

            // block already valid, do not rewrite
            if existing_hash == original_hash {
                progress.inc(1);
                return Ok(());
            }

            loop {
                // Write
                dst_handle.seek(SeekFrom::Start(offset))?;
                dst_handle.write_all(&buffer)?;
                dst_handle.flush()?;

                // Verify
                let mut verify_buf = vec![0u8; buffer.len()];
                dst_handle.seek(SeekFrom::Start(offset))?;
                dst_handle.read_exact(&mut verify_buf)?;

                let verify_hash = Sha256::digest(&verify_buf);

                if verify_hash == original_hash {
                    progress.inc(1);
                    break;
                } else {
                    log::warning(format!(
                        "🟥 Block {} failed verification. Retrying...",
                        block_index
                    ))?;
                }
            }

            Ok(())
        })?;

    progress.stop("✅ All blocks verified and valid.");
    outro("🏁 Mission complete. The file is safe.")?;
    Ok(())
}
