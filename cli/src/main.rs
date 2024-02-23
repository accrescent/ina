// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    fs::{self, File},
    io,
    io::Read,
    path::PathBuf,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use ina::Patcher;

#[derive(Parser)]
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
        Command::Diff { old, new, patch } => {
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

            ina::diff(&old_data, &new_data, &mut patch_file)
                .context("I/O error occurred while generating patch file")?;
        }
        Command::Patch { old, patch, out } => {
            let old_file = File::open(&old)
                .with_context(|| format!("Failed to open old file '{}'", old.display()))?;
            let patch_file = File::open(&patch)
                .with_context(|| format!("Failed to open patch file '{}'", patch.display()))?;
            let mut out_file = File::create(&out)
                .with_context(|| format!("Failed to create out file '{}'", out.display()))?;

            let mut patcher =
                Patcher::new(old_file, patch_file).context("Failed to read patch file")?;
            io::copy(&mut patcher, &mut out_file)
                .context("I/O error occurred while applying patch file")?;
        }
    }

    Ok(())
}
