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

const ZSTD_COMPRESSION_LEVEL: i32 = 19;

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
    // Write the header
    patch.write_u32::<LittleEndian>(MAGIC)?;
    patch.write_u32::<LittleEndian>(VERSION)?;

    // Create a compressor for the inner patch data
    let mut patch_encoder = Encoder::new(patch, ZSTD_COMPRESSION_LEVEL)?;

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
