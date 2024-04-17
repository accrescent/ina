// Copyright 2023-2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use alloc::{vec, vec::Vec};
use core::mem;

// This algorithm casts u32s to usizes for the purpose of indexing. Because of these casts, any
// target where the size of a usize is less than the size of a u32 will produce unexpected (albeit
// not undefined) behavior. To prevent this, cause a compiler error on such targets.
#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("Target pointer width must be at least 32 bits");

/// The size of the alphabet
const ALPHABET_SIZE: usize = 256;

/// The representation of an empty value
const EMPTY: u32 = 1 << (u32::BITS - 1);

/// Computes the suffix array of `data` using the SACA-K algorithm.
///
/// The algorithm is implemented as described in the [article] Practical Linear-Time O(1)-Workspace
/// Suffix Sorting for Constant Alphabets by Ge Nong. Obviously, it runs in *O*(*n*) time.
///
/// One of the constraints of the SACA-K algorithm as implemented is that the last element in
/// `data` must be 0. However, unlike the presentation of the algorithm in the original article,
/// this element does not need to be unique, i.e., other elements may also be 0. As a result, this
/// function can correctly construct suffix arrays for arbitrary byte strings.
///
/// # Panics
///
/// Panics if the last element in `data` is not 0.
///
/// [article]: https://doi.org/10.1145/2493175.2493180
pub(crate) fn sacak(data: &[u8]) -> Vec<u32> {
    if data.is_empty() {
        Vec::new()
    } else {
        assert_eq!(data[data.len() - 1], 0, "last element in `data` must be 0");

        let mut suffix_array = vec![0; data.len()];

        if data.len() != 1 {
            sacak_level_zero(data, &mut suffix_array);
        }

        suffix_array
    }
}

fn sacak_level_zero(data: &[u8], suffix_array: &mut [u32]) {
    let mut bucket = vec![0; ALPHABET_SIZE];

    // Stage 1: Reduce the problem by at least 1/2
    put_substring_zero(suffix_array, data, &mut bucket);
    induce_suffix_array_l_zero(suffix_array, data, &mut bucket, false);
    induce_suffix_array_s_zero(suffix_array, data, &mut bucket, false);

    // At this point, all the LMS-substrings are sorted and stored sparsely in the suffix array
    // space.
    //
    // Compact all of the sorted substrings into the first n1 items of the suffix array space.
    let mut n1: u32 = 0;
    for i in 0..data.len() {
        if suffix_array[i] > 0 {
            suffix_array[n1 as usize] = suffix_array[i];
            n1 += 1;
        }
    }

    let s1_offset = suffix_array.len() as u32 - n1;
    let name_counter = name_substrings_zero(suffix_array, data, n1, s1_offset);

    // Stage 2: Solve the reduced problem

    // Recurse if the names are not yet unique
    if name_counter < n1 {
        let (suffix_array, data) = suffix_array.split_at_mut(suffix_array.len() - n1 as usize);
        sacak_recursive(suffix_array, bytemuck::cast_slice::<u32, u8>(data));
    } else {
        // Get the suffix array of s1 directly
        for i in 0..n1 {
            suffix_array[suffix_array[(s1_offset + i) as usize] as usize] = i;
        }
    }

    // Stage 3: Induce SA(S) from SA(S1)
    get_suffix_array_lms_zero(suffix_array, data, n1, s1_offset);

    put_suffix_zero(suffix_array, data, &mut bucket, n1);
    induce_suffix_array_l_zero(suffix_array, data, &mut bucket, true);
    induce_suffix_array_s_zero(suffix_array, data, &mut bucket, true);
}

fn sacak_recursive(suffix_array: &mut [u32], data: &[u8]) {
    put_substring_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
    );
    induce_suffix_array_l_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
        false,
    );
    induce_suffix_array_s_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
        false,
    );

    // At this point, all the LMS-substrings are sorted and stored sparsely in the suffix array
    // space.
    //
    // Compact all of the sorted substrings into the first n1 items of the suffix array space.
    let mut n1: u32 = 0;
    for i in 0..(data.len() / 4) {
        if bytemuck::cast_slice::<u32, i32>(suffix_array)[i] > 0 {
            suffix_array[n1 as usize] = suffix_array[i];
            n1 += 1;
        }
    }

    let s1_offset = suffix_array.len() as u32 - n1;
    let name_counter = name_substrings_one(suffix_array, data, n1, s1_offset);

    // Stage 2: Solve the reduced problem

    // Recurse if the names are not yet unique
    if name_counter < n1 {
        let (suffix_array, data) = suffix_array.split_at_mut(suffix_array.len() - n1 as usize);
        sacak_recursive(suffix_array, bytemuck::cast_slice::<u32, u8>(data));
    } else {
        // Get the suffix array of s1 directly
        for i in 0..n1 {
            suffix_array[suffix_array[(s1_offset + i) as usize] as usize] = i;
        }
    }

    // Stage 3: Induce SA(S) from SA(S1)
    get_suffix_array_lms_one(suffix_array, data, n1, s1_offset);

    put_suffix_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
        n1,
    );
    induce_suffix_array_l_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
        true,
    );
    induce_suffix_array_s_one(
        bytemuck::cast_slice_mut::<u32, i32>(suffix_array),
        bytemuck::cast_slice::<u8, i32>(data),
        true,
    );
}

fn put_suffix_one(suffix_array: &mut [i32], data: &[i32], n1: u32) {
    let mut pre: i32 = -1;
    let mut pos: i32 = 0;

    for i in (1..=(n1 - 1)).rev() {
        let j = suffix_array[i as usize];
        suffix_array[i as usize] = EMPTY as i32;
        let cur = data[j as usize];
        if cur != pre {
            pre = cur;
            pos = cur;
        }
        suffix_array[pos as usize] = j;
        pos -= 1;
    }
}

fn get_suffix_array_lms_one(suffix_array: &mut [u32], data: &[u8], n1: u32, s1_offset: u32) {
    let mut j: u32 = n1 - 1;
    suffix_array[(s1_offset + j) as usize] = (data.len() / 4) as u32 - 1;
    j = j.wrapping_sub(1);

    // data[n - 2] is L-type by definition
    let mut successive_type = CharType::L;

    for i in (1..=(data.len() / 4 - 2)).rev() {
        let current_type = if bytemuck::cast_slice::<u8, i32>(data)[i - 1]
            < bytemuck::cast_slice::<u8, i32>(data)[i]
            || (bytemuck::cast_slice::<u8, i32>(data)[i - 1]
                == bytemuck::cast_slice::<u8, i32>(data)[i]
                && successive_type == CharType::S)
        {
            CharType::S
        } else {
            CharType::L
        };
        if current_type == CharType::L && successive_type == CharType::S {
            suffix_array[(s1_offset + j) as usize] = i as u32;
            j = j.wrapping_sub(1);
        }
        successive_type = current_type;
    }

    for i in 0..n1 {
        suffix_array[i as usize] = suffix_array[(s1_offset + suffix_array[i as usize]) as usize];
    }

    // Initialize suffix_array[n1..(n - 1)]
    for x in suffix_array
        .iter_mut()
        .take(data.len() / 4)
        .skip(n1 as usize)
    {
        *x = EMPTY;
    }
}

fn get_length_of_lms_one(data: &[u8], x: u32) -> u32 {
    if x == (data.len() / 4) as u32 - 1 {
        return 1;
    }

    let mut i: u32 = 1;
    let mut dist: u32 = 0;
    loop {
        if bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize]
            < bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize - 1]
        {
            break;
        }
        i += 1;
    }
    loop {
        if x + i > data.len() as u32 / 4 - 1
            || bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize]
                > bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize - 1]
        {
            break;
        }
        if x + i == data.len() as u32 / 4 - 1
            || bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize]
                < bytemuck::cast_slice::<u8, i32>(data)[(x + i) as usize - 1]
        {
            dist = i;
        }
        i += 1;
    }

    dist + 1
}

fn name_substrings_one(suffix_array: &mut [u32], data: &[u8], n1: u32, s1_offset: u32) -> u32 {
    // Initialize the name array buffer
    for x in suffix_array
        .iter_mut()
        .take(data.len() / 4)
        .skip(n1 as usize)
    {
        *x = EMPTY;
    }

    // Scan to compute the interim s1
    let mut name_counter: u32 = 0;
    let mut name: u32 = 0;
    let mut pre_len: u32 = 0;
    let mut pre_pos: u32 = 0;
    for i in 0..n1 {
        let mut diff = false;
        let pos = suffix_array[i as usize];

        let len = get_length_of_lms_one(data, pos);
        if len != pre_len {
            diff = true;
        } else {
            for d in 0..len {
                if pos + d == (data.len() / 4) as u32 - 1
                    || pre_pos + d == (data.len() / 4) as u32 - 1
                    || bytemuck::cast_slice::<u8, i32>(data)[(pos + d) as usize]
                        != bytemuck::cast_slice::<u8, i32>(data)[(pre_pos + d) as usize]
                {
                    diff = true;
                    break;
                }
            }
        }

        if diff {
            name = i;
            name_counter += 1;
            // A new name
            suffix_array[name as usize] = 1;
            pre_pos = pos;
            pre_len = len;
        } else {
            // Count this name
            suffix_array[name as usize] += 1;
        }

        suffix_array[(n1 + pos / 2) as usize] = name;
    }

    // Compact the interim s1 sparsely stored in suffix_array[n1, n - 1] into
    // suffix_array[m - n1, m - 1]
    let mut i: u32 = (data.len() / 4) as u32 - 1;
    let mut j: u32 = suffix_array.len() as u32 - 1;
    while i >= n1 {
        if suffix_array[i as usize] != EMPTY {
            suffix_array[j as usize] = suffix_array[i as usize];
            j -= 1;
        }

        i -= 1;
    }

    // Rename each S-type character of the interim s1 as the end of its bucket to produce the final
    // s1
    let mut successive_type = CharType::S;
    let mut i: u32 = n1 - 1;
    while i > 0 {
        let ch: i32 = suffix_array[(s1_offset + i) as usize] as i32;
        let ch1: i32 = suffix_array[(s1_offset + i - 1) as usize] as i32;
        let current_type = if ch1 < ch || (ch1 == ch && successive_type == CharType::S) {
            CharType::S
        } else {
            CharType::L
        };
        if current_type == CharType::S {
            suffix_array[(s1_offset + i) as usize - 1] +=
                suffix_array[suffix_array[(s1_offset + i) as usize - 1] as usize] - 1;
        }
        successive_type = current_type;

        i -= 1;
    }

    name_counter
}

fn induce_suffix_array_s_one(suffix_array: &mut [i32], data: &[i32], suffix: bool) {
    let mut i: i32 = data.len() as i32 - 1;
    let mut step: i32;
    while i > 0 {
        step = 1;
        let j = suffix_array[i as usize].wrapping_sub(1);
        if suffix_array[i as usize] <= 0 {
            i -= step;
            continue;
        }
        let c = data[j as usize];
        let c1 = data[(j + 1) as usize];
        let is_s = c < c1 || (c == c1 && c > i);
        if !is_s {
            i -= step;
            continue;
        }

        // suffix_array[j] is S-type
        let mut d = suffix_array[c as usize];
        if d >= 0 {
            // suffix_array[c] is borrowed by the right neigbor bucket. Shift-right the items in
            // the right neighbor bucket.
            let mut tmp = suffix_array[c as usize];
            let mut h: i32 = c + 1;
            while suffix_array[h as usize] >= 0 || suffix_array[h as usize] == EMPTY as i32 {
                mem::swap(&mut suffix_array[h as usize], &mut tmp);

                h += 1;
            }
            suffix_array[h as usize] = tmp;
            if h > i {
                step = 0;
            }

            d = EMPTY as i32;
        }

        if d == EMPTY as i32 {
            // suffix_array[c] is empty
            if suffix_array[(c - 1) as usize] == EMPTY as i32 {
                // Initialize the counter
                suffix_array[c as usize] = -1;
                suffix_array[(c - 1) as usize] = j;
            } else {
                // A size-1 bucket
                suffix_array[c as usize] = j;
            }
        } else {
            // suffix_array[c] is reused as a counter
            let mut pos: i32 = c + d - 1;
            if suffix_array[pos as usize] != EMPTY as i32 {
                // We are running into the left neighbor bucket. Shift-right one step the items of
                // bucket(suffix_array, data, j).
                let mut h: i32 = 0;
                while h < -d {
                    suffix_array[(c - h) as usize] = suffix_array[(c - h - 1) as usize];

                    h += 1;
                }
                pos += 1;
                if c > i {
                    step = 0;
                }
            } else {
                suffix_array[c as usize] -= 1;
            }

            suffix_array[pos as usize] = j;
        }

        if !suffix {
            let i1 = if step == 0 { i + 1 } else { i };
            suffix_array[i1 as usize] = EMPTY as i32;
        }

        i -= step;
    }

    // Scan to shift-right the items in each bucket with its head being reused as a counter
    if !suffix {
        for i in (1..=(data.len() - 1)).rev() {
            let j = suffix_array[i];
            // Is suffix_array[i] a counter?
            if j < 0 && j != EMPTY as i32 {
                let mut h: i32 = 0;
                while h < -j {
                    suffix_array[i - h as usize] = suffix_array[i - h as usize - 1];

                    h += 1;
                }
                suffix_array[i - h as usize] = EMPTY as i32;
            }
        }
    }
}

fn induce_suffix_array_l_one(suffix_array: &mut [i32], data: &[i32], suffix: bool) {
    let mut i: i32 = 0;
    let mut step: i32;
    while i < data.len() as i32 {
        step = 1;
        let j = suffix_array[i as usize].wrapping_sub(1);
        if suffix_array[i as usize] <= 0 {
            i += step;
            continue;
        }
        let c = data[j as usize];
        let c1 = data[(j + 1) as usize];
        let is_l = c >= c1;
        if !is_l {
            i += step;
            continue;
        }

        // data[j] is L-type

        let mut d = suffix_array[c as usize];
        if d >= 0 {
            // suffix_array[c] is borrowed by the left neighbor bucket. Shift-left the items in the
            // left neighbor bucket.
            let mut tmp = suffix_array[c as usize];
            let mut h: i32 = c - 1;
            while suffix_array[h as usize] >= 0 || suffix_array[h as usize] == EMPTY as i32 {
                mem::swap(&mut suffix_array[h as usize], &mut tmp);

                h -= 1;
            }
            suffix_array[h as usize] = tmp;
            if h < i {
                step = 0;
            }

            d = EMPTY as i32;
        }

        if d == EMPTY as i32 {
            // suffix_array[c] is empty
            if c < data.len() as i32 - 1 && suffix_array[(c + 1) as usize] == EMPTY as i32 {
                // Initialize the counter
                suffix_array[c as usize] = -1;
                suffix_array[(c + 1) as usize] = j;
            } else {
                // A size-1 bucket
                suffix_array[c as usize] = j;
            }
        } else {
            // suffix_array[c] is reused as a counter
            let mut pos: i32 = c - d + 1;
            if pos > data.len() as i32 - 1 || suffix_array[pos as usize] != EMPTY as i32 {
                // We are running into the right neighbor bucket. Shift-left one step the items of
                // bucket(suffix_array, daat, j).
                let mut h: i32 = 0;
                while h < -d {
                    suffix_array[(c + h) as usize] = suffix_array[(c + h + 1) as usize];

                    h += 1;
                }
                pos -= 1;
                if c < i {
                    step = 0;
                }
            } else {
                suffix_array[c as usize] -= 1;
            }

            suffix_array[pos as usize] = j;
        }

        // Is data[suffix_array[i]] L-type?
        let c2: i32;
        let is_l = (j + 1 < data.len() as i32 - 1) && {
            c2 = data[(j + 2) as usize];
            c1 > c2 || (c1 == c2 && c1 < i)
        };
        if (!suffix || !is_l) && i > 0 {
            let i1: i32 = if step == 0 { i - 1 } else { i };
            suffix_array[i1 as usize] = EMPTY as i32;
        }

        i += step;
    }

    // Scan to shift-left the items in each bucket with its head being reused as a counter
    for i in 1..data.len() {
        let j = suffix_array[i];
        if j < 0 && j != EMPTY as i32 {
            let mut h: i32 = 0;
            while h < -j {
                suffix_array[i + h as usize] = suffix_array[i + h as usize + 1];

                h += 1;
            }
            suffix_array[i + h as usize] = EMPTY as i32;
        }
    }
}

fn put_substring_one(suffix_array: &mut [i32], data: &[i32]) {
    for x in suffix_array.iter_mut().take(data.len()) {
        *x = EMPTY as i32;
    }

    let mut c1: i32 = data[data.len() - 2];
    let mut t1 = false;
    for i in (1..=(data.len() - 2)).rev() {
        let c = c1;
        let t = t1;
        c1 = data[i - 1];
        t1 = c1 < c || (c1 == c && t);
        if t && !t1 {
            if suffix_array[c as usize] >= 0 {
                // suffix_array[c] is borrowed by the right neighbor bucket. Shift-right the items
                // in the right neighbor bocket.
                let mut tmp: i32 = suffix_array[c as usize];
                let mut h: i32 = c + 1;
                while suffix_array[h as usize] >= 0 {
                    mem::swap(&mut suffix_array[h as usize], &mut tmp);

                    h += 1;
                }
                suffix_array[h as usize] = tmp;

                suffix_array[c as usize] = EMPTY as i32;
            }

            let d = suffix_array[c as usize];
            if d == EMPTY as i32 {
                if suffix_array[(c - 1) as usize] == EMPTY as i32 {
                    // Initialize the counter
                    suffix_array[c as usize] = -1;

                    suffix_array[(c - 1) as usize] = i as i32;
                } else {
                    // A size-1 bucket
                    suffix_array[c as usize] = i as i32;
                }
            } else {
                // suffix_array[c] is reused as a counter
                let mut pos: i32 = c + d - 1;
                if suffix_array[pos as usize] != EMPTY as i32 {
                    // We are running into the left neighbor bucket. Shift-right one step the items
                    // of bucket(suffix_array, data, i).
                    let mut h: i32 = 0;
                    while h < -d {
                        suffix_array[(c - h) as usize] = suffix_array[(c - h - 1) as usize];

                        h += 1;
                    }
                    pos += 1;
                } else {
                    suffix_array[c as usize] -= 1;
                }

                suffix_array[pos as usize] = i as i32;
            }
        }
    }

    // Scan to shift-right the items in each bucket with its head being reused as a counter
    for i in (1..=(data.len() - 1)).rev() {
        let j = suffix_array[i];

        // Is suffix_array[i] a counter?
        if j < 0 && j != EMPTY as i32 {
            let mut h: i32 = 0;
            while h < -j {
                suffix_array[(i as i32 - h) as usize] = suffix_array[(i as i32 - h - 1) as usize];

                h += 1;
            }
            suffix_array[(i as i32 - h) as usize] = EMPTY as i32;
        }
    }

    // Put the single sentinel LMS-substring
    suffix_array[0] = data.len() as i32 - 1;
}

fn put_suffix_zero(suffix_array: &mut [u32], data: &[u8], bucket: &mut [u32], n1: u32) {
    // Find the end of each bucket
    get_buckets(data, bucket, true);

    // Put the suffixes into their buckets
    for i in (1..=(n1 - 1)).rev() {
        let j: u32 = suffix_array[i as usize];
        suffix_array[i as usize] = 0;
        suffix_array[bucket[data[j as usize] as usize] as usize] = j;
        bucket[data[j as usize] as usize] -= 1;
    }

    // Set the single sentinel suffix
    suffix_array[0] = data.len() as u32 - 1;
}

fn get_suffix_array_lms_zero(suffix_array: &mut [u32], data: &[u8], n1: u32, s1_offset: u32) {
    let mut j: u32 = n1 - 1;
    suffix_array[(s1_offset + j) as usize] = data.len() as u32 - 1;
    j = j.wrapping_sub(1);

    // data[n - 2] is L-type by definition
    let mut successive_type = CharType::L;

    for i in (1..=(data.len() - 2)).rev() {
        let current_type = if data[i - 1] < data[i]
            || (data[i - 1] == data[i] && successive_type == CharType::S)
        {
            CharType::S
        } else {
            CharType::L
        };
        if current_type == CharType::L && successive_type == CharType::S {
            suffix_array[(s1_offset + j) as usize] = i as u32;
            j = j.wrapping_sub(1);
        }
        successive_type = current_type;
    }

    for i in 0..n1 {
        suffix_array[i as usize] = suffix_array[(s1_offset + suffix_array[i as usize]) as usize];
    }

    // Initialize suffix_array[n1..(n - 1)]
    for x in suffix_array.iter_mut().take(data.len()).skip(n1 as usize) {
        *x = 0;
    }
}

fn name_substrings_zero(suffix_array: &mut [u32], data: &[u8], n1: u32, s1_offset: u32) -> u32 {
    // Initialize the name array buffer
    for x in suffix_array.iter_mut().take(data.len()).skip(n1 as usize) {
        *x = EMPTY;
    }

    // Scan to compute the interim s1
    let mut name_counter: u32 = 0;
    let mut name: u32 = 0;
    let mut pre_len: u32 = 0;
    let mut pre_pos: u32 = 0;
    for i in 0..n1 {
        let mut diff = false;
        let pos = suffix_array[i as usize];

        let len = get_length_of_lms_zero(data, pos);
        if len != pre_len {
            diff = true;
        } else {
            for d in 0..len {
                if pos + d == data.len() as u32 - 1
                    || pre_pos + d == data.len() as u32 - 1
                    || data[(pos + d) as usize] != data[(pre_pos + d) as usize]
                {
                    diff = true;
                    break;
                }
            }
        }

        if diff {
            name = i;
            name_counter += 1;
            // A new name
            suffix_array[name as usize] = 1;
            pre_pos = pos;
            pre_len = len;
        } else {
            // Count this name
            suffix_array[name as usize] += 1;
        }

        suffix_array[(n1 + pos / 2) as usize] = name;
    }

    // Compact the interim s1 sparsely stored in suffix_array[n1, n - 1] into
    // suffix_array[m - n1, m - 1]
    let mut i: u32 = data.len() as u32 - 1;
    let mut j: u32 = suffix_array.len() as u32 - 1;
    while i >= n1 {
        if suffix_array[i as usize] != EMPTY {
            suffix_array[j as usize] = suffix_array[i as usize];
            j -= 1;
        }

        i -= 1;
    }

    // Rename each S-type character of the interim s1 as the end of its bucket to produce the final
    // s1
    let mut successive_type = CharType::S;
    let mut i: u32 = n1 - 1;
    while i > 0 {
        let ch: i32 = suffix_array[(s1_offset + i) as usize] as i32;
        let ch1: i32 = suffix_array[(s1_offset + i - 1) as usize] as i32;
        let current_type = if ch1 < ch || (ch1 == ch && successive_type == CharType::S) {
            CharType::S
        } else {
            CharType::L
        };
        if current_type == CharType::S {
            suffix_array[(s1_offset + i) as usize - 1] +=
                suffix_array[suffix_array[(s1_offset + i) as usize - 1] as usize] - 1;
        }
        successive_type = current_type;

        i -= 1;
    }

    name_counter
}

fn get_length_of_lms_zero(data: &[u8], x: u32) -> u32 {
    if x == data.len() as u32 - 1 {
        return 1;
    }

    let mut i: u32 = 1;
    let mut dist: u32 = 0;
    loop {
        if data[(x + i) as usize] < data[(x + i) as usize - 1] {
            break;
        }
        i += 1;
    }
    loop {
        if x + i > data.len() as u32 - 1 || data[(x + i) as usize] > data[(x + i) as usize - 1] {
            break;
        }
        if x + i == data.len() as u32 - 1 || data[(x + i) as usize] < data[(x + i) as usize - 1] {
            dist = i;
        }
        i += 1;
    }

    dist + 1
}

fn induce_suffix_array_s_zero(
    suffix_array: &mut [u32],
    data: &[u8],
    bucket: &mut [u32],
    suffix: bool,
) {
    get_buckets(data, bucket, true);

    for i in (1..=(data.len() - 1)).rev() {
        if suffix_array[i] > 0 {
            let j = suffix_array[i] as usize - 1;
            if data[j] <= data[j + 1] && bucket[data[j] as usize] < i as u32 {
                suffix_array[bucket[data[j] as usize] as usize] = j as u32;
                bucket[data[j] as usize] -= 1;
                if !suffix {
                    suffix_array[i] = 0;
                }
            }
        }
    }
}

fn induce_suffix_array_l_zero(
    suffix_array: &mut [u32],
    data: &[u8],
    bucket: &mut [u32],
    suffix: bool,
) {
    get_buckets(data, bucket, false);

    // Skip the virtual sentinel
    bucket[0] += 1;

    for i in 0..data.len() {
        if suffix_array[i] > 0 {
            let j = suffix_array[i] as usize - 1;
            if data[j] >= data[j + 1] {
                suffix_array[bucket[data[j] as usize] as usize] = j as u32;
                bucket[data[j] as usize] += 1;
                if !suffix && i > 0 {
                    suffix_array[i] = 0;
                }
            }
        }
    }
}

fn put_substring_zero(suffix_array: &mut [u32], data: &[u8], bucket: &mut [u32]) {
    get_buckets(data, bucket, true);

    // The penultimate element in `data` is L-type by definition
    let mut successive_type = CharType::L;

    for i in (1..=(data.len() - 2)).rev() {
        let current_type = if data[i - 1] < data[i]
            || (data[i - 1] == data[i] && successive_type == CharType::S)
        {
            CharType::S
        } else {
            CharType::L
        };
        if current_type == CharType::L && successive_type == CharType::S {
            suffix_array[bucket[data[i] as usize] as usize] = i as u32;
            bucket[data[i] as usize] -= 1;
        }
        successive_type = current_type;
    }

    // Set the single sentinel LMS-substring
    suffix_array[0] = data.len() as u32 - 1;
}

#[derive(PartialEq)]
enum CharType {
    L,
    S,
}

fn get_buckets(data: &[u8], bucket: &mut [u32], end: bool) {
    // Clear all buckets
    for x in bucket.iter_mut() {
        *x = 0;
    }

    // Compute the size of each bucket
    for x in data.iter() {
        bucket[*x as usize] += 1;
    }

    // Calculate bucket ends or bucket starts into `bucket` if `end` is true or false respectively
    let mut sum: u32 = 0;
    for x in bucket.iter_mut() {
        sum += *x;
        *x = if end { sum - 1 } else { sum - *x }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_non_recursive_string() {
        let text = "Hello, world!\0";
        let suffix_array = sacak(text.as_bytes());

        assert_eq!(
            &suffix_array,
            &[13, 6, 12, 5, 0, 11, 1, 10, 2, 3, 4, 8, 9, 7],
        );
    }

    #[test]
    fn multiple_zeroes() {
        let text = "Hello, \0world!\0";
        let suffix_array = sacak(text.as_bytes());

        assert_eq!(
            &suffix_array,
            &[14, 7, 6, 13, 5, 0, 12, 1, 11, 2, 3, 4, 9, 10, 8],
        );
    }

    #[test]
    fn empty_string() {
        let text = "";
        let suffix_array = sacak(text.as_bytes());

        assert_eq!(&suffix_array, &[]);
    }

    #[test]
    fn only_sentinel() {
        let text = "\0";
        let suffix_array = sacak(text.as_bytes());

        assert_eq!(&suffix_array, &[0]);
    }
}
