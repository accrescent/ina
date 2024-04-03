// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use ina::DiffConfig;

/// Binary diffing and patching designed for executables
#[derive(Parser)]
#[command(display_name("ina"), version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Diff {
        old: PathBuf,
        new: PathBuf,
        patch: PathBuf,
        #[arg(long)]
        compression_threads: Option<u32>,
        #[arg(long)]
        compression_level: Option<i32>,
    },
    Patch {
        old: PathBuf,
        patch: PathBuf,
        out: PathBuf,
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
        Command::Patch { old, patch, out } => {
            let old_file = File::open(&old)
                .with_context(|| format!("Failed to open old file '{}'", old.display()))?;
            let patch_file = File::open(&patch)
                .with_context(|| format!("Failed to open patch file '{}'", patch.display()))?;
            let mut out_file = File::create(&out)
                .with_context(|| format!("Failed to create out file '{}'", out.display()))?;

            ina::patch(old_file, patch_file, &mut out_file)
                .context("Failed to apply patch file")?;
        }
    }

    Ok(())
}
