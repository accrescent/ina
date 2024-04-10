// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    cmp,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, BufRead, BufReader, ErrorKind, Read, Seek, SeekFrom, Write},
};

use byteorder::{LittleEndian, ReadBytesExt};
use integer_encoding::VarIntReader;
use zstd::Decoder;

use crate::header::{MAGIC, VERSION_MAJOR};

const DEFAULT_BUF_SIZE: usize = 8192;

/// A patcher that reconstructs a new blob from an old blob and a patch
///
/// Because this struct implements [`Read`], it can be used to apply a patch in a streaming
/// fashion, e.g., while reading the patch from the network.
pub struct Patcher<'a, O, B>
where
    O: Read + Seek,
    B: BufRead,
{
    old: O,
    patch: Decoder<'a, B>,
    state: PatcherState,
    buf: Vec<u8>,
}

enum PatcherState {
    AtNextControl,
    Add(usize),
    Copy(usize),
}

impl<'a, O, B> Patcher<'a, O, B>
where
    O: Read + Seek,
    B: BufRead,
{
    /// Creates a new `Patcher` for `old` and `patch` using a pre-existing buffer.
    ///
    /// Each `Patcher` uses an internal read buffer for decompression. [`Patcher::new()`] optimizes
    /// the size of this read buffer for the decompression algorithm used, so it's highly
    /// recommended to use that method to create a `Patcher` instead of this one when possible.
    /// However, this method may be useful if you need to set a hard limit on `Patcher` memory
    /// usage or make allocations upfront for sandboxing purposes.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the patch metadata or if the patch
    /// metadata is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::{fs::File, io::BufReader};
    /// use ina::Patcher;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Ensure our Patcher never uses more than 4 MiB of memory
    /// const BUF_SIZE: usize = 1 << 22;
    ///
    /// let old = File::open("app-v1.exe")?;
    /// let patch = File::open("app-v1-to-v2.ina")?;
    ///
    /// let patcher = Patcher::with_buffer(old, BufReader::with_capacity(BUF_SIZE, patch))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_buffer(old: O, mut patch: B) -> Result<Self, PatchError> {
        read_header(&mut patch)?;

        let patch_decoder = Decoder::with_buffer(patch)?;

        Ok(Self {
            old,
            patch: patch_decoder,
            state: PatcherState::AtNextControl,
            buf: vec![0; DEFAULT_BUF_SIZE],
        })
    }
}

impl<'a, O, P> Patcher<'a, O, BufReader<P>>
where
    O: Read + Seek,
    P: Read,
{
    /// Creates a new `Patcher` for `old` and `patch`.
    ///
    /// Each `Patcher` uses an internal read buffer for decompression. When using this method to
    /// create a `Patcher`, the size of this buffer is optimized for the decompression algorithm
    /// used, so it's highly recommended to use this method for creating a `Patcher` in most
    /// circumstances. If you need to supply your own buffer, use [`Patcher::with_buffer()`]
    /// instead.
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
        read_header(&mut patch)?;

        let patch_decoder = Decoder::new(patch)?;

        Ok(Self {
            old,
            patch: patch_decoder,
            state: PatcherState::AtNextControl,
            buf: vec![0; DEFAULT_BUF_SIZE],
        })
    }
}

impl<'a, O, B> Read for Patcher<'a, O, B>
where
    O: Read + Seek,
    B: BufRead,
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
                    // We're currently reading an add field, so read `len` bytes from both the old
                    // file and the patch file, add them together, and write the result to the
                    // buffer.
                    //
                    // Because `buf` may not be large enough to hold everything we need to read, we
                    // keep track of how many bytes we wrote and jump back to this state if needed.
                    let max_read_len = cmp::min(cmp::min(add_len, buf.len()), self.buf.len());

                    let out = &mut buf[..max_read_len];
                    self.old.read_exact(out)?;

                    // Reuse `self.buf` to hold the difference bytes read from the patch file
                    // without allocating on every `read()`
                    let diff = &mut self.buf[..max_read_len];
                    self.patch.read_exact(diff)?;

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
    /// The patch major version is unsupported
    UnsupportedVersion(u16),
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
                    "unsupported version: found version {version}.x, \
                    supported versions are {VERSION_MAJOR}.x",
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

impl From<TryFromValueError> for PatchError {
    fn from(value: TryFromValueError) -> Self {
        PatchError::UnsupportedVersion(value.0)
    }
}

/// Metadata of a patch file.
///
/// This struct represents information about a patch file present in its header such the patch
/// format version.
pub struct PatchMetadata {
    version: PatchVersion,
}

impl PatchMetadata {
    fn new(version: PatchVersion) -> Self {
        Self { version }
    }

    /// Returns the version of the patch file format.
    pub fn version(&self) -> PatchVersion {
        self.version
    }
}

/// Version of a patch file format.
///
/// This structure represents an acceptable patch format version which we know how to parse.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct PatchVersion {
    major: MajorVersion,
    minor: u16,
}

impl PatchVersion {
    fn from_values(major: u16, minor: u16) -> Result<Self, TryFromValueError> {
        let major = major.try_into()?;

        Ok(Self { major, minor })
    }

    /// Returns the major version of the patch format
    pub fn major(&self) -> u16 {
        self.major.into()
    }

    /// Returns the minor version of the patch format
    pub fn minor(&self) -> u16 {
        self.minor
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
enum MajorVersion {
    One,
}

impl TryFrom<u16> for MajorVersion {
    type Error = TryFromValueError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MajorVersion::One),
            _ => Err(TryFromValueError(value)),
        }
    }
}

impl From<MajorVersion> for u16 {
    fn from(value: MajorVersion) -> Self {
        match value {
            MajorVersion::One => 1,
        }
    }
}

#[derive(Debug)]
struct TryFromValueError(u16);

impl Display for TryFromValueError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "version out of supported range")
    }
}

impl Error for TryFromValueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Reads the header of `patch` to extract its metadata.
///
/// This function reads the full header of `patch`, including fields the current parser doesn't
/// understand. This behavior means that the `patch` reader will always point to the beginning of
/// the patch data section after successful completion of this function.
///
/// # Errors
///
/// Returns an error if an I/O error occurs while reading the patch metadata or if the patch
/// metadata is invalid.
pub fn read_header<P>(patch: &mut P) -> Result<PatchMetadata, PatchError>
where
    P: Read,
{
    let magic = patch.read_u32::<LittleEndian>()?;
    if magic != MAGIC {
        return Err(PatchError::BadMagic(magic));
    }

    let version_major = patch.read_u16::<LittleEndian>()?;
    let version_minor = patch.read_u16::<LittleEndian>()?;
    let patch_version = PatchVersion::from_values(version_major, version_minor)?;

    let data_offset = patch.read_varint()?;

    // Discard the portion of the patch we don't understand
    io::copy(&mut patch.take(data_offset), &mut io::sink())?;

    Ok(PatchMetadata::new(patch_version))
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
