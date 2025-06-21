// Copyright 2023-2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

#![allow(missing_docs)]

use std::{fs::File, io::Read};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use sufsort::SuffixArray;

const DATA_PATH: &str = "benches/testdata/pizzachili-pitches.data";
const CHUNK_SIZE: u64 = 512;

fn construct(c: &mut Criterion) {
    let mut group = c.benchmark_group("construct");

    let mut file = File::open(DATA_PATH).unwrap();
    let file_size = file.metadata().unwrap().len();
    // Add one to the buffer size to account for the sentinel
    let mut contents = Vec::with_capacity((file_size + 1).try_into().unwrap());

    // Split file into chunks to test multiple data sizes
    let mut chunk_sizes = Vec::new();
    let mut bytes_left = file_size;
    while bytes_left > 0 {
        if bytes_left >= CHUNK_SIZE {
            chunk_sizes.push(CHUNK_SIZE);
            bytes_left -= CHUNK_SIZE;
        } else {
            chunk_sizes.push(bytes_left);
            bytes_left = 0;
        }
    }

    let mut cumulative = 0;
    for size in chunk_sizes {
        cumulative += size;

        // Append another chunk to our data buffer
        file.by_ref().take(size).read_to_end(&mut contents).unwrap();

        // Add a sentinel
        contents.push(0);

        group
            .throughput(Throughput::Bytes(cumulative + 1))
            .bench_with_input(
                BenchmarkId::from_parameter(cumulative),
                &contents,
                |b, data| {
                    b.iter(|| SuffixArray::new(data));
                },
            );

        // Remove the sentinel so we can append further to the buffer
        contents.pop();
    }

    group.finish();
}

criterion_group!(benches, construct);
criterion_main!(benches);
