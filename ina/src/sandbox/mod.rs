// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Sandboxing utilities for Ina operations.
//!
//! This module contains functions to enable platform-specific sandboxing that is guaranteed to be
//! compatible with Ina's operations. They are abstract over the platform targeted, enabling
//! appropriate sandboxing on platforms with supported sandboxing methods on a best-effort basis,
//! so it's recommended to call the respective sandboxing functions on all targets whenever
//! possible to automatically take advantage of additional platform sandbox support.
//!
//! The methods are separated by the operation being performed since patching and diffing may use
//! different platform capabilities.
//!
//! # Examples
//!
//! ```no_run
//! use std::fs::File;
//! use ina::sandbox;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Perform setup for patching before enabling the sandbox
//! let old = File::open("app-v1.exe")?;
//! let patch = File::open("app-v1-to-v2.ina")?;
//! let mut new = File::create("app-v2.exe")?;
//!
//! // Enable the platform's sandbox for patching
//! sandbox::enable_for_patching()?;
//!
//! // Patch the blob
//! ina::patch(old, patch, &mut new)?;
//! # Ok(())
//! # }
//! ```

pub use seccompiler;

mod common;
mod patch;

pub use common::SandboxError;
pub use patch::enable as enable_for_patching;
