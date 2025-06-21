// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

#![allow(missing_docs)]

use std::{
    error::Error,
    fs::{self, File},
    io,
    path::Path,
};

use blake3::Hasher;

const OLD_FILE_NAME: &str = "gcc-13.1.1";
const NEW_FILE_NAME: &str = "gcc-13.2.1";
const PATCH_FILE_NAME: &str = "gcc-13.1.1-13.2.1.ina";

#[test]
fn gcc() -> Result<(), Box<dyn Error>> {
    let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata");
    let workspace_dir = Path::new(env!("CARGO_TARGET_TMPDIR"));

    // Create a patch file
    {
        let mut old = fs::read(test_data_dir.join(OLD_FILE_NAME))?;
        // Add a sentinel so the algorithm works properly
        old.push(0);
        let new = fs::read(test_data_dir.join(NEW_FILE_NAME))?;
        let mut patch = File::create(workspace_dir.join(PATCH_FILE_NAME))?;
        ina::diff(&old, &new, &mut patch)?;
    }

    // Reconstruct the new file from the old file and the patch file
    {
        let old = File::open(test_data_dir.join(OLD_FILE_NAME))?;
        let patch = File::open(workspace_dir.join(PATCH_FILE_NAME))?;
        let mut new = File::create(workspace_dir.join(NEW_FILE_NAME))?;
        ina::patch(old, patch, &mut new)?;
    }

    // Verify that patching worked correctly by comparing the hashes of the new and reconstructed
    // new files
    let mut new = File::open(test_data_dir.join(NEW_FILE_NAME))?;
    let mut reconstructed_new = File::open(workspace_dir.join(NEW_FILE_NAME))?;

    let mut new_hasher = Hasher::new();
    let mut reconstructed_new_hasher = Hasher::new();
    io::copy(&mut new, &mut new_hasher)?;
    io::copy(&mut reconstructed_new, &mut reconstructed_new_hasher)?;
    let new_hash = new_hasher.finalize();
    let reconstructed_new_hash = reconstructed_new_hasher.finalize();

    assert_eq!(new_hash, reconstructed_new_hash);

    Ok(())
}
