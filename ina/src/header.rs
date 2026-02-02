// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: Apache-2.0

pub(crate) const MAGIC: u32 = 0x5c956c7c;
pub(crate) const VERSION_MAJOR: u16 = 1;
#[cfg(feature = "diff")]
pub(crate) const VERSION_MINOR: u16 = 0;
#[cfg(feature = "diff")]
pub(crate) const DATA_OFFSET: u16 = 0;
