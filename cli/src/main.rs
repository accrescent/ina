// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    fs::{self, File},
    io::{self, BufReader, Read},
    path::PathBuf,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use ina::{DiffConfig, Patcher};

/// Binary diffing and patching designed for executables
#[derive(Parser)]
#[command(display_name("ina"), version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a patch between two files
    Diff {
        /// The path of the old file
        old: PathBuf,
        /// The path of the new file
        new: PathBuf,
        /// The path of the output patch file
        patch: PathBuf,
        /// The number of threads to use for compression
        ///
        /// Setting this to a value more than 0 allows compression to run on a separate thread than
        /// I/O, significantly improving performance at a slight cost to maximum memory usage.
        /// Values above 1 result in greatly diministing returns, so the default is recommended
        /// unless testing proves higher performance with higher values.
        ///
        /// A value of 0 means that compression will run on the same thread as I/O, reducing
        /// diffing speed but slightly lowering memory usage.
        ///
        /// Default: 1
        #[arg(long, verbatim_doc_comment)]
        compression_threads: Option<u32>,
        /// The compression level to use for compressing the patch file
        ///
        /// The compression level can be set to any value between -7 and 22 inclusive. The most
        /// positive number results in the highest compression ratio at the cost of speed, while
        /// the least positive number results in the highest speed at the cost of compression
        /// ratio. Any value outside of this range will be clamped to fit inside the range.
        ///
        /// Levels 20-22 result in significantly higher memory usage.
        ///
        /// Default: 19
        #[arg(long, verbatim_doc_comment)]
        compression_level: Option<i32>,
    },
    /// Reconstruct a new file from and old file and a patch
    Patch {
        /// The path of the old file
        old: PathBuf,
        /// The path of the patch file
        patch: PathBuf,
        /// The path of the output new file
        new: PathBuf,
        /// The size in bytes of the buffer to use for decompression
        ///
        /// By default, the patching process creates an internal read buffer whose size is
        /// optimized for the decompression algorithm in use. Because it is optimized, it is
        /// recommended to leave it at its default size unless there is a specific reason to change
        /// it. Low values may reduce memory usage at a cost of patching speed.
        ///
        /// Default: varies
        #[arg(long, verbatim_doc_comment)]
        decompression_buffer_size: Option<usize>,
    },
    /// Display patch metadata
    Info {
        /// The path of the patch file
        patch: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Diff {
            old,
            new,
            patch,
            compression_threads,
            compression_level,
        } => {
            let mut old_file = File::open(&old)
                .with_context(|| format!("Failed to open old file '{}'", old.display()))?;
            let len: usize = old_file
                .metadata()
                .with_context(|| {
                    format!("Failed to read metadata of old file '{}'", old.display())
                })?
                .len()
                .try_into()
                .with_context(|| {
                    format!(
                        "Old file '{}' is too large to read into memory",
                        old.display(),
                    )
                })?;
            // Reserve a byte of extra space for the sentinel
            let mut old_data = Vec::with_capacity(len + 1);
            old_file
                .read_to_end(&mut old_data)
                .context("Failure occurred while reading old file")?;
            // Last byte must be 0
            old_data.push(0);

            let new_data = fs::read(&new)
                .with_context(|| format!("Failed to read new file '{}'", new.display()))?;

            let mut patch_file = File::create(&patch)
                .with_context(|| format!("Failed to create patch file '{}'", patch.display()))?;

            let mut diff_config = DiffConfig::default();
            if let Some(threads) = compression_threads {
                diff_config.compression_threads(threads);
            }
            if let Some(level) = compression_level {
                diff_config.compression_level(level);
            }

            ina::diff_with_config(&old_data, &new_data, &mut patch_file, &diff_config)
                .context("I/O error occurred while generating patch file")?;
        }
        Command::Patch {
            old,
            patch,
            new,
            decompression_buffer_size,
        } => {
            let old_file = File::open(&old)
                .with_context(|| format!("Failed to open old file '{}'", old.display()))?;
            let patch_file = File::open(&patch)
                .with_context(|| format!("Failed to open patch file '{}'", patch.display()))?;
            let mut new_file = File::create(&new)
                .with_context(|| format!("Failed to create new file '{}'", new.display()))?;

            let mut patcher = match decompression_buffer_size {
                Some(size) => {
                    Patcher::with_buffer(old_file, BufReader::with_capacity(size, patch_file))?
                }
                None => Patcher::new(old_file, patch_file)?,
            };
            io::copy(&mut patcher, &mut new_file).context("Failed to apply patch file")?;
        }
        Command::Info { patch } => {
            let mut patch_file = File::open(&patch)
                .with_context(|| format!("Failed to open patch file '{}'", patch.display()))?;

            let patch_format_version = ina::read_header(&mut patch_file)
                .with_context(|| format!("Failed to read patch header of '{}'", patch.display()))?
                .version();

            println!(
                "Ina patch file, format version {}.{}",
                patch_format_version.major(),
                patch_format_version.minor(),
            );
        }
    }

    Ok(())
}
