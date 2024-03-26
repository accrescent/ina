// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    cmp,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom, Write},
};

use byteorder::{LittleEndian, ReadBytesExt};
use integer_encoding::VarIntReader;
use zstd::Decoder;

use crate::header::{MAGIC, VERSION};

/// A patcher that reconstructs a new blob from an old blob and a patch
///
/// Because this struct implements [`Read`], it can be used to apply a patch in a streaming
/// fashion, e.g., while reading the patch from the network.
pub struct Patcher<'a, O, P>
where
    O: Read + Seek,
    P: Read,
{
    old: O,
    patch: Decoder<'a, BufReader<P>>,
    state: PatcherState,
}

enum PatcherState {
    AtNextControl,
    Add(usize),
    Copy(usize),
}

/// An error indicating that patching a blob failed.
///
/// This error is returned by [`Patcher::new()`] when the patch given to it contains invalid
/// metadata or reading the metadata fails. For more information, see that function's
/// documentation.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use ina::{PatchError, Patcher};
///
/// let mut old = Cursor::new(&[1, 2, 3, 4]);
/// // Garbage data
/// let patch = &[0, 0, 0, 0];
/// let patcher = Patcher::new(old, patch.as_ref());
///
/// assert!(matches!(patcher, Err(PatchError::BadMagic(_))));
/// ```
#[derive(Debug)]
pub enum PatchError {
    /// An I/O error occurred
    Io(io::Error),
    /// The patch magic is invalid
    BadMagic(u32),
    /// The patch version is unsupported
    UnsupportedVersion(u32),
}

impl Display for PatchError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            PatchError::Io(e) => write!(f, "I/O error: {e}"),
            PatchError::BadMagic(magic) => {
                write!(f, "bad magic: expected {MAGIC:x}, found {magic:x}")
            }
            PatchError::UnsupportedVersion(version) => {
                write!(
                    f,
                    "unsupported version: found {version}, supported versions are [{VERSION}]",
                )
            }
        }
    }
}

impl Error for PatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PatchError::Io(e) => e.source(),
            _ => None,
        }
    }
}

impl From<io::Error> for PatchError {
    fn from(value: io::Error) -> Self {
        PatchError::Io(value)
    }
}

impl<'a, O, P> Patcher<'a, O, P>
where
    O: Read + Seek,
    P: Read,
{
    /// Creates a new `Patcher` for `old` and `patch`.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the patch metadata or if the patch
    /// metadata is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use ina::Patcher;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let old = File::open("app-v1.exe")?;
    /// let patch = File::open("app-v1-to-v2.ina")?;
    ///
    /// let patcher = Patcher::new(old, patch)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(old: O, mut patch: P) -> Result<Self, PatchError> {
        let magic = patch.read_u32::<LittleEndian>()?;
        if magic != MAGIC {
            return Err(PatchError::BadMagic(magic));
        }

        let version = patch.read_u32::<LittleEndian>()?;
        if version != VERSION {
            return Err(PatchError::UnsupportedVersion(version));
        }

        let patch_decoder = Decoder::new(patch)?;

        Ok(Self {
            old,
            patch: patch_decoder,
            state: PatcherState::AtNextControl,
        })
    }
}

impl<'a, O, P> Read for Patcher<'a, O, P>
where
    O: Read + Seek,
    P: Read,
{
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let mut read_total = 0;

        while !buf.is_empty() {
            let read = match self.state {
                PatcherState::AtNextControl => {
                    // Next is a control add field. Read the length of it and continue.
                    match self.patch.read_varint() {
                        Ok(add_len) => {
                            self.state = PatcherState::Add(add_len);
                            0
                        }
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => break,
                            _ => return Err(e),
                        },
                    }
                }
                PatcherState::Add(add_len) => {
                    // We're currently reading an add field, so read `len` bytes from both the old file
                    // and the patch file, add them together, and write the result to the buffer.
                    //
                    // Because `buf` may not be large enough to hold everything we need to read, we
                    // keep track of how many bytes we wrote and jump back to this state if needed.
                    let max_read_len = cmp::min(add_len, buf.len());

                    let out = &mut buf[..max_read_len];
                    self.old.read_exact(out)?;

                    let mut diff = vec![0; max_read_len];
                    self.patch.read_exact(&mut diff)?;

                    (0..max_read_len).for_each(|i| out[i] = out[i].wrapping_add(diff[i]));

                    if add_len == max_read_len {
                        // We finished reading all of the add bytes, so read the copy field len and
                        // transition to the copy reading state
                        let copy_len = self.patch.read_varint()?;
                        self.state = PatcherState::Copy(copy_len);
                    } else {
                        // We didn't read all of the add bytes, so continue to do so on the next read
                        // iteration
                        self.state = PatcherState::Add(add_len - max_read_len);
                    }

                    max_read_len
                }
                PatcherState::Copy(copy_len) => {
                    // We're currently reading a copy field, so write the next bytes into the buffer
                    // directly.
                    //
                    // Again, `buf` may not be large enough to hold everything we need to read, so we
                    // keep track of how many bytes we wrote and jump back to this state if needed.
                    let max_read_len = cmp::min(copy_len, buf.len());

                    let out = &mut buf[..max_read_len];
                    self.patch.read_exact(out)?;

                    if copy_len == max_read_len {
                        // We finished reading the copy field, so perform a seek and jump to reading
                        // the next add field
                        let seek = self.patch.read_varint()?;
                        self.old.seek(SeekFrom::Current(seek))?;

                        self.state = PatcherState::AtNextControl;
                    } else {
                        self.state = PatcherState::Copy(copy_len - max_read_len);
                    }

                    max_read_len
                }
            };

            read_total += read;
            buf = &mut buf[read..];
        }

        Ok(read_total)
    }
}

/// Reconstructs a new blob from an old blob and a patch
///
///
/// This is a convenience method for creating a [`Patcher`] and reading it to completion. If
/// successful, returns the number of bytes written to `new`.
///
/// # Errors
///
/// Returns an error if an I/O occurs while reading the patch metadata of if the patch metadata is
/// invalid.
///
/// # Examples
///
/// ```no_run
/// use std::fs::File;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let old = File::open("app-v1.exe")?;
/// let patch = File::open("app-v1-to-v2.ina")?;
/// let mut new = File::create("app-v2.exe")?;
///
/// ina::patch(old, patch, &mut new)?;
///
/// # Ok(())
/// # }
/// ```
pub fn patch<O, P, W>(old: O, patch: P, new: &mut W) -> Result<u64, PatchError>
where
    O: Read + Seek,
    P: Read,
    W: Write + ?Sized,
{
    let mut patcher = Patcher::new(old, patch)?;

    Ok(io::copy(&mut patcher, new)?)
}
