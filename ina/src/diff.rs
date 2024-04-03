// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::io::{self, Write};

use byteorder::{LittleEndian, WriteBytesExt};
use integer_encoding::VarIntWriter;
use zstd::Encoder;

use crate::{
    bsdiff::ControlProducer,
    header::{MAGIC, VERSION},
};

/// Constructs a patch between two blobs with default options
///
/// Note that `old` MUST have a `0` appended to the end of the actual old blob for the algorithm to
/// work properly.
///
/// The diffing algorithm used works on arbitrary blobs, but is designed for and particularly
/// well-suited for creating small patch files between native executables.
///
/// The resulting data written to `patch` can later be applied to `old` to reconstruct `new` by
/// using a [`Patcher`](crate::Patcher).
///
/// This function is a shorthand for [`diff_with_config()`] called with the default options. If you
/// want to tune the algorithm configuration, see that function instead.
///
/// # Errors
///
/// Returns an error if an I/O error occurs while writing the patch.
///
/// # Panics
///
/// Panics if the last element of `old` is not 0.
///
/// # Examples
///
/// ```
/// # fn main() -> std::io::Result<()> {
/// let old = b"Hello\0";
/// let new = b"Hero";
/// let mut patch = Vec::new();
///
/// ina::diff(old, new, &mut patch)?;
///
/// # Ok(())
/// # }
/// ```
pub fn diff<W>(old: &[u8], new: &[u8], patch: &mut W) -> io::Result<()>
where
    W: Write + ?Sized,
{
    diff_with_config(old, new, patch, &DiffConfig::default())
}

/// Constructs a patch between two blobs
///
/// Note that `old` MUST have a `0` appended to the end of the actual old blob for the algorithm to
/// work properly.
///
/// The diffing algorithm used works on arbitrary blobs, but is designed for and particularly
/// well-suited for creating small patch files between native executables.
///
/// The resulting data written to `patch` can later be applied to `old` to reconstruct `new` by
/// using a [`Patcher`](crate::Patcher).
///
/// # Errors
///
/// Returns an error if an I/O error occurs while writing the patch.
///
/// # Panics
///
/// Panics if the last element of `old` is not 0.
///
/// # Examples
///
/// ```
/// # fn main() -> std::io::Result<()> {
/// use ina::DiffConfig;
///
/// let old = b"Hello\0";
/// let new = b"Hero";
/// let mut patch = Vec::new();
///
/// ina::diff_with_config(old, new, &mut patch, &DiffConfig::new().compression_threads(0))?;
///
/// # Ok(())
/// # }
/// ```
pub fn diff_with_config<W>(
    old: &[u8],
    new: &[u8],
    patch: &mut W,
    options: &DiffConfig,
) -> io::Result<()>
where
    W: Write + ?Sized,
{
    // Write the header
    patch.write_u32::<LittleEndian>(MAGIC)?;
    patch.write_u32::<LittleEndian>(VERSION)?;

    // Create a compressor for the inner patch data
    let mut patch_encoder = Encoder::new(patch, options.compression_level)?;
    patch_encoder.multithread(options.compression_threads)?;

    // Iterate over bsdiff control values, writing them to the patch stream
    for control in ControlProducer::new(old, new) {
        // Write add section
        patch_encoder.write_varint(control.add().len())?;
        patch_encoder.write_all(control.add())?;

        // Write copy section
        patch_encoder.write_varint(control.copy().len())?;
        patch_encoder.write_all(control.copy())?;

        // Write seek value
        patch_encoder.write_varint(control.seek())?;
    }

    patch_encoder.finish()?;

    Ok(())
}

/// Configuration for a diff operation.
///
/// This struct can be used to fine-tune parameters to the diffing algorithm. The defaults should
/// be optimal for most use cases, but you may wish to change them in especially
/// resource-constrained or powerful computing environments for better performance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct DiffConfig {
    compression_threads: u32,
    compression_level: i32,
}

impl DiffConfig {
    /// Creates a new configuration for diff operations
    ///
    /// This configuration can be reused across diff operations.
    pub const fn new() -> Self {
        Self {
            compression_threads: Self::DEFAULT_COMPRESSION_THREADS,
            compression_level: Self::DEFAULT_COMPRESSION_LEVEL,
        }
    }

    /// Sets the number of threads to use for compressing the patch file.
    ///
    /// Setting this to a value more than 0 allows compression to run on a separate thread than
    /// I/O, significantly improving performance at a slight cost to maximum memory usage. Values
    /// above 1 result in greatly diminishing returns, so the default is recommended unless testing
    /// proves higher performance with higher values.
    ///
    /// A value of 0 means that compression will run on the same thread as I/O, reducing diffing
    /// speed but slightly lowering memory usage.
    pub fn compression_threads(&mut self, threads: u32) -> &mut Self {
        self.compression_threads = threads;
        self
    }

    /// Sets the compression level to use for compressing the patch file.
    ///
    /// The compression level can be set to any value between -7 and 22 inclusive. The most
    /// positive number results in the highest compression ratio at the cost of speed, while the
    /// least positive number results in the highest speed at the cost of compression ratio. Any
    /// value outside of this range will be clamped to fit inside the range.
    ///
    /// Levels 20-22 result in significantly higher memory usage.
    pub fn compression_level(&mut self, level: i32) -> &mut Self {
        self.compression_level = level;
        self
    }

    /// The default number of compression threads to create
    ///
    /// We set this to 1 to ensure I/O and compression can run concurrently.
    pub const DEFAULT_COMPRESSION_THREADS: u32 = 1;

    /// The default compression level to use
    ///
    /// We set this to 19 because it obtains the highest compression ratio without incurring the
    /// significant memory costs of higher levels.
    pub const DEFAULT_COMPRESSION_LEVEL: i32 = 19;
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self::new()
    }
}
