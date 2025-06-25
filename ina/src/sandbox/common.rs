// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// An error indicating that sandboxing the process failed.
///
/// This error is returned by [`enable_for_patching()`] when enabling the platform's sandbox fails.
///
/// The set of potential errors is expected to grow as sandboxing support for more platforms is
/// added.
///
/// [`enable_for_patching()`]: super::enable_for_patching
#[derive(Debug)]
#[non_exhaustive]
pub enum SandboxError {
    /// A seccomp error occurred
    Seccomp(seccompiler::Error),
}

impl Display for SandboxError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SandboxError::Seccomp(e) => write!(f, "seccomp error: {e}"),
        }
    }
}

impl Error for SandboxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SandboxError::Seccomp(e) => e.source(),
        }
    }
}

impl From<seccompiler::Error> for SandboxError {
    fn from(value: seccompiler::Error) -> Self {
        SandboxError::Seccomp(value)
    }
}
