// Copyright 2023 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Suffix array construction for byte strings.
//!
//! This crate provides suffix arrays for byte strings, which can be used as indexes for efficient
//! substring searching.
//!
//! # Examples
//!
//! Creating a suffix array for an ASCII string:
//!
//! ```
//! use sufsort::SuffixArray;
//!
//! // Notice the null byte at the end of the string
//! let data = b"Hello, world!\0";
//! let suffix_array = SuffixArray::new(data);
//!
//! assert!(suffix_array.contains(b"Hello"));
//! ```
//!
//! Creating a suffix array for a binary file:
//!
//! ```no_run
//! use std::{fs::File, io::Read};
//!
//! use sufsort::SuffixArray;
//!
//! let mut file = File::open("image.bmp")?;
//! let size = file.metadata()?.len();
//! // + 1 to account for the sentinel
//! let mut contents = Vec::with_capacity((size + 1).try_into().unwrap());
//!
//! file.read_to_end(&mut contents)?;
//!
//! // Add the required sentinel
//! contents.push(0);
//!
//! let suffix_array = SuffixArray::new(&contents);
//!
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//!
//! # Construction
//!
//! The construction algorithm runs in *O*(*n*) time and *O*(1) space for strings of length *n*
//! (this space excluding the sizes of the input data and output suffix array, which total 5*n*
//! bytes). All searching operations run in *O*(*m* \* log(*n*)) time for patterns of length *m*.
//!
//! # Design considerations
//!
//! This library has a very strong focus on security, robustness, and speed. As such, it is:
//!
//! - Written in 100% safe Rust
//! - Rigorously tested
//! - Carefully benchmarked
//!
//! `sufsort` doesn't aim to be a comprehensive suffix sorting library since its primary purpose is
//! to serve as a step in another algorithm. However, it in no way strives to be minimalist and
//! will freely take on additional features as needed.

#![no_std]

extern crate alloc;

mod sacak;
mod suffix_array;

pub use suffix_array::SuffixArray;
