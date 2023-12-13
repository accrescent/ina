// Copyright 2023 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use crate::sacak;

/// A suffix array for a byte string.
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
        assert_eq!(data[data.len() - 1], 0);

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
}
