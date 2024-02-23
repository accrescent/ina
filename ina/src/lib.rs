// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Binary diffing and patching designed for executables.
//!
//! This crate provides simple interfaces for creating binary deltas between arbitrary blobs and
//! applying them to reconstruct the original blob. It is especially well-suited for executable
//! files, i.e., it is designed to produce small deltas specifically for executables.
//!
//! # Examples
//!
//! Creating a patch file between two executable versions:
//!
//! ```no_run
//! use std::fs::{self, File};
//!
//! # fn main() -> std::io::Result<()> {
//! let mut old = fs::read("app-v1.exe")?;
//! // Ensure the last byte is a 0
//! old.push(0);
//! let new = fs::read("app-v2.exe")?;
//! let mut patch = File::create("app-v1-to-v2.ina")?;
//!
//! ina::diff(&old, &new, &mut patch)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! Applying a patch file to create an updated executable:
//!
//! ```no_run
//! use std::{io, fs::File};
//! use ina::Patcher;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let old = File::open("app-v1.exe")?;
//! let patch = File::open("app-v1-to-v2.ina")?;
//! let mut new = File::create("app-v2.exe")?;
//!
//! ina::patch(old, patch, &mut new)?;
//!
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "diff")]
mod bsdiff;
#[cfg(feature = "diff")]
mod diff;
#[cfg(any(feature = "diff", feature = "patch"))]
mod header;
#[cfg(feature = "patch")]
mod patch;

#[cfg(feature = "diff")]
pub use diff::diff;
#[cfg(feature = "patch")]
pub use patch::{patch, PatchError, Patcher};
