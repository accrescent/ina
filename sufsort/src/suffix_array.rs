// Copyright 2023 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

use alloc::vec::Vec;
use core::{cmp::Ordering, ops::Deref};

use crate::sacak;

/// A suffix array for a byte string.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SuffixArray<'a> {
    data: &'a [u8],
    inner: Vec<u32>,
}

impl<'a> SuffixArray<'a> {
    /// Creates a new `SuffixArray` for `data`.
    ///
    /// Note that `data` MUST have a `0` appended to the end of the data you actually wish to sort
    /// for the algorithm to work properly.
    ///
    /// This operation is *O*(*n*).
    ///
    /// # Panics
    ///
    /// Panics if the last element in `data` is not 0 or if `data.len() > u32::MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sufsort::SuffixArray;
    ///
    /// let data = b"Hello, world!\0";
    /// let sa = SuffixArray::new(data);
    /// ```
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        let inner = sacak::sacak(data);

        Self { data, inner }
    }

    /// Returns `true` if and only if `pattern` is contained in the associated data.
    ///
    /// This operation is *O*(*m* \* log(*n*)), where `m` is `pattern.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sufsort::SuffixArray;
    ///
    /// let data = b"Hello, world!\0";
    /// let sa = SuffixArray::new(data);
    /// assert!(sa.contains(b"world"));
    /// ```
    #[must_use]
    pub fn contains(&self, pattern: &[u8]) -> bool {
        self.inner
            .binary_search_by(|&suffix| {
                self.data[suffix as usize..]
                    .iter()
                    .take(pattern.len())
                    .cmp(pattern.iter())
            })
            .is_ok()
    }

    /// Returns the longest substring of the associated data that matches a prefix of `pattern`.
    ///
    /// Returns `None` if no matching suffix is found.
    ///
    /// This operation runs in *O*(*m* \* log(*n*)) time, where `m` is `pattern.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sufsort::SuffixArray;
    ///
    /// let data = b"Red fish\0";
    /// let sa = SuffixArray::new(data);
    ///
    /// assert_eq!(sa.longest_match(b"fish").as_deref(), Some(b"fish".as_ref()));
    /// assert_eq!(sa.longest_match(b"fishes").as_deref(), Some(b"fish".as_ref()));
    /// assert_eq!(sa.longest_match(b"zebra").as_deref(), None);
    /// assert_eq!(sa.longest_match(b"find").as_deref(), Some(b"fi".as_ref()));
    /// assert_eq!(sa.longest_match(b"Red fish\0 swim").as_deref(), Some(b"Red fish\0".as_ref()));
    /// ```
    #[must_use]
    pub fn longest_match(&self, pattern: &[u8]) -> Option<Substring<'_>> {
        macro_rules! suffix {
            ($i: expr) => {
                &self.data[$i as usize..]
            };
        }

        macro_rules! len {
            ($i: expr) => {
                common_prefix_len(suffix!($i), pattern)
            };
        }

        macro_rules! substring {
            ($position: expr, $len: expr) => {
                Substring {
                    position: $position,
                    data: &self.data[$position..($position + $len)],
                }
            };
        }

        // Binary search our suffixes to find a match for `pattern`
        let search_result = self
            .inner
            .binary_search_by(|&suffix_index| {
                suffix!(suffix_index)
                    .iter()
                    .take(pattern.len())
                    .cmp(pattern.iter())
            })
            .map(|i| self.inner[i] as usize);

        match search_result {
            Ok(position) => Some(substring!(position, len!(position))),
            Err(sorted_pos) => {
                // The full pattern wasn't found, meaning that either:
                //
                // 1. A partial match was found in a sorted suffix ot the left or right side of
                //    `sorted_pos`.
                // 2. No match was found whatsoever.
                //
                // Therefore, find the longest common prefix lengths between the pattern and the
                // sorted suffixes to the left and right of our position to determine which one
                // contains the longest match.
                //
                // The presence of the sentinel guarantees 1 <= `sorted_pos` <= data.len(), so the
                // following subtractions should never underflow.
                let left_lcp_len = len!(self.inner[sorted_pos - 1]);
                let right_lcp_len = self.inner.get(sorted_pos).map_or(0, |p| len!(*p));

                match left_lcp_len.cmp(&right_lcp_len) {
                    Ordering::Less => {
                        Some(substring!(self.inner[sorted_pos] as usize, right_lcp_len))
                    }
                    Ordering::Equal => {
                        // It doesn't matter whether we use left_lcp_len or right_lcp_len here, so
                        // choose left_lcp_len arbitrarily
                        if left_lcp_len == 0 {
                            None
                        } else {
                            Some(substring!(
                                self.inner[sorted_pos - 1] as usize,
                                left_lcp_len
                            ))
                        }
                    }
                    Ordering::Greater => Some(substring!(
                        self.inner[sorted_pos - 1] as usize,
                        left_lcp_len
                    )),
                }
            }
        }
    }
}

fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

/// A substring of a sorted text.
///
/// # Examples
///
/// ```
/// use std::ops::Deref;
/// use sufsort::SuffixArray;
///
/// let suffix_array = SuffixArray::new(b"Hello, world!\0");
/// let substring = suffix_array.longest_match(b"worth").unwrap();
///
/// assert_eq!(substring.deref(), b"wor");
/// assert_eq!(substring.position(), 7);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Substring<'a> {
    position: usize,
    data: &'a [u8],
}

impl<'a> Substring<'a> {
    /// Returns the index of the first character of the substring in the original text.
    ///
    /// # Examples
    ///
    /// ```
    /// use sufsort::SuffixArray;
    ///
    /// let suffix_array = SuffixArray::new(b"Roses are red\0");
    /// let substring = suffix_array.longest_match(b"are blue").unwrap();
    ///
    /// assert_eq!(substring.position(), 6);
    /// ```
    pub fn position(&self) -> usize {
        self.position
    }
}

impl<'a> Deref for Substring<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_one_match() {
        let data = b"Hello, world!\0";
        let sa = SuffixArray::new(data);

        assert!(sa.contains(b"world"));
    }

    #[test]
    fn contains_two_matches() {
        let data = b"The quick brown fox jumped over the lazy dog because the fox was quick\0";
        let sa = SuffixArray::new(data);

        assert!(sa.contains(b"fox"));
        assert!(sa.contains(b"quick"));
    }

    #[test]
    fn contains_no_matches() {
        let data = b"Now is the time for all good men to come to the aid of the party\0";
        let sa = SuffixArray::new(data);

        assert!(!sa.contains(b"times"));
    }

    #[test]
    #[should_panic]
    fn no_sentinel() {
        let data = b"Hello, world!";
        let _ = SuffixArray::new(data);
    }

    #[test]
    fn full_substring_match() {
        let data = b"Provident totam et illum esse qui voluptas corrupti.\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b"illum").unwrap();

        assert_eq!(substring.position(), 19);
        assert_eq!(substring.deref(), b"illum");
    }

    #[test]
    fn partial_substring_match_first_suffix() {
        let data = b"Hello, world!\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b" worth").unwrap();

        assert_eq!(substring.position(), 6);
        assert_eq!(substring.deref(), b" wor");
    }

    #[test]
    fn partial_substring_match_middle_suffix() {
        let data = b"The quick brown fox jumped over the lazy dog\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b"brown dog").unwrap();

        assert_eq!(substring.position(), 10);
        assert_eq!(substring.deref(), b"brown ");
    }

    #[test]
    fn partial_substring_match_last_suffix() {
        let data = b"Hello, world!\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b"worlds").unwrap();

        assert_eq!(substring.position(), 7);
        assert_eq!(substring.deref(), b"world");
    }

    #[test]
    fn no_substring_match() {
        let data = b"Hello, world!\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b"zebra");

        assert_eq!(substring, None);
    }

    #[test]
    fn substring_match_longer_pattern() {
        let data = b"Red fish\0";
        let sa = SuffixArray::new(data);
        let substring = sa.longest_match(b"fish\0are blue").unwrap();

        assert_eq!(substring.position(), 4);
        assert_eq!(substring.deref(), b"fish\0");
    }
}
