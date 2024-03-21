// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use sufsort::SuffixArray;

const NON_MATCHING_BYTES_THRESHOLD: usize = 8;

#[derive(Clone, Copy)]
pub(crate) struct Match {
    add_old_pos: usize,
    add_new_pos: usize,
    add_len: usize,
    copy_end: usize,
}

impl Match {
    fn copy_pos(&self) -> usize {
        self.add_new_pos + self.add_len
    }
}

pub(crate) struct MatchMaker<'a> {
    scan: usize,
    len: usize,
    pos: usize,
    last_scan: usize,
    last_pos: usize,
    last_offset: isize,
    old: &'a [u8],
    new: &'a [u8],
    old_index: SuffixArray<'a>,
}

impl<'a> MatchMaker<'a> {
    fn new(old: &'a [u8], new: &'a [u8]) -> Self {
        let old_index = SuffixArray::new(old);

        Self {
            scan: 0,
            len: 0,
            pos: 0,
            last_scan: 0,
            last_pos: 0,
            last_offset: 0,
            old,
            new,
            old_index,
        }
    }
}

impl<'a> Iterator for MatchMaker<'a> {
    type Item = Match;

    fn next(&mut self) -> Option<Self::Item> {
        while self.scan < self.new.len() {
            let mut old_score = 0;
            self.scan += self.len;
            let mut scsc = self.scan;
            while self.scan < self.new.len() {
                (self.pos, self.len) = self
                    .old_index
                    .longest_match(&self.new[self.scan..])
                    .map(|s| (s.position(), s.len()))
                    .unwrap_or((0, 0));

                while scsc < self.scan + self.len {
                    if ((scsc as isize + self.last_offset) as usize) < self.old.len()
                        && self.old[(scsc as isize + self.last_offset) as usize] == self.new[scsc]
                    {
                        old_score += 1;
                    }
                    scsc += 1;
                }

                if (self.len == old_score && self.len != 0)
                    || self.len > old_score + NON_MATCHING_BYTES_THRESHOLD
                {
                    break;
                }

                if ((self.scan as isize + self.last_offset) as usize) < self.old.len()
                    && self.old[(self.scan as isize + self.last_offset) as usize]
                        == self.new[self.scan]
                {
                    old_score -= 1;
                }

                self.scan += 1;
            }

            if self.len != old_score || self.scan == self.new.len() {
                let mut s = 0;
                let mut s_f = 0;
                let mut len_forward: usize = 0;
                let mut i = 0;
                while self.last_scan + i < self.scan && self.last_pos + i < self.old.len() {
                    if self.old[self.last_pos + i] == self.new[self.last_scan + i] {
                        s += 1;
                    }
                    i += 1;
                    if s * 2 - i as isize > s_f * 2 - len_forward as isize {
                        s_f = s;
                        len_forward = i;
                    }
                }

                let mut len_back = 0;
                if self.scan < self.new.len() {
                    let mut s = 0;
                    let mut s_b = 0;
                    let mut i = 0;
                    while self.scan >= self.last_scan + i && self.pos >= i {
                        if self.old[self.pos - i] == self.new[self.scan - i] {
                            s += 1;
                        }
                        if s * 2 - i as isize > s_b * 2 - len_back as isize {
                            s_b = s;
                            len_back = i;
                        }

                        i += 1;
                    }
                }

                if self.last_scan + len_forward > self.scan - len_back {
                    let overlap = (self.last_scan + len_forward) - (self.scan - len_back);
                    let mut s = 0;
                    let mut s_s = 0;
                    let mut lens = 0;
                    let mut i = 0;
                    while i < overlap {
                        if self.new[self.last_scan + len_forward - overlap + i]
                            == self.old[self.last_pos + len_forward - overlap + i]
                        {
                            s += 1;
                        }
                        if self.new[self.scan - len_back + i] == self.old[self.pos - len_back + i] {
                            s -= 1;
                        }
                        if s > s_s {
                            s_s = s;
                            lens = i + 1;
                        }

                        i += 1;
                    }

                    len_forward += lens;
                    len_forward -= overlap;
                    len_back -= lens;
                }

                // Yield a match
                let bsdiff_match = Match {
                    add_old_pos: self.last_pos,
                    add_new_pos: self.last_scan,
                    add_len: len_forward,
                    copy_end: self.scan - len_back,
                };

                self.last_scan = self.scan - len_back;
                self.last_pos = self.pos - len_back;
                self.last_offset = self.pos as isize - self.scan as isize;

                return Some(bsdiff_match);
            }
        }

        None
    }
}

pub(crate) struct Control<'a> {
    add: Vec<u8>,
    copy: &'a [u8],
    seek: i64,
}

impl<'a> Control<'a> {
    pub(crate) fn add(&self) -> &[u8] {
        &self.add
    }

    pub(crate) fn copy(&self) -> &'a [u8] {
        self.copy
    }

    pub(crate) fn seek(&self) -> i64 {
        self.seek
    }
}

pub(crate) struct ControlProducer<'a, I>
where
    I: Iterator<Item = Match>,
{
    match_iter: I,
    prev_match: Option<Match>,
    old: &'a [u8],
    new: &'a [u8],
}

impl<'a> ControlProducer<'a, MatchMaker<'a>> {
    pub(crate) fn new(old: &'a [u8], new: &'a [u8]) -> Self {
        let match_iter = MatchMaker::new(old, new);

        Self {
            match_iter,
            prev_match: None,
            old,
            new,
        }
    }
}

impl<'a, I> Iterator for ControlProducer<'a, I>
where
    I: Iterator<Item = Match>,
{
    type Item = Control<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.prev_match {
                Some(prev_match) => {
                    let add = (0..prev_match.add_len)
                        .map(|i| {
                            self.new[prev_match.add_new_pos + i]
                                .wrapping_sub(self.old[prev_match.add_old_pos + i])
                        })
                        .collect();
                    let copy = &self.new[prev_match.copy_pos()..prev_match.copy_end];

                    self.prev_match = self.match_iter.next();

                    let seek = self.prev_match.map_or(0, |m| {
                        m.add_old_pos as i64 - (prev_match.add_old_pos + prev_match.add_len) as i64
                    });

                    break Some(Control { add, copy, seek });
                }
                None => {
                    self.prev_match = self.match_iter.next();
                    if self.prev_match.is_none() {
                        break None;
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}
