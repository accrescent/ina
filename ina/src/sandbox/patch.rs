// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use super::common::SandboxError;

/// Enables the platform-specific sandbox for patching
///
/// Returns `Ok(true)` if sandboxing was successfully enabled for the current platform and
/// `Ok(false)` if no supported sandboxing method was detected.
///
/// # Errors
///
/// Returns an error if a supported sandboxing method is detected on the current platform, but
/// enabling it fails.
///
/// # Examples
///
/// ```no_run
/// use std::fs::File;
/// use ina::sandbox;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Perform setup for patching before enabling the sandbox
/// let old = File::open("app-v1.exe")?;
/// let patch = File::open("app-v1-to-v2.ina")?;
/// let mut new = File::create("app-v2.exe")?;
///
/// // Enable the platform's sandbox for patching
/// sandbox::enable_for_patching()?;
///
/// // Patch the blob
/// ina::patch(old, patch, &mut new)?;
/// # Ok(())
/// # }
/// ```
pub fn enable() -> Result<bool, SandboxError> {
    Ok(enable_platform_sandbox()?)
}

#[cfg(all(
    target_os = "android",
    target_endian = "little",
    any(target_arch = "aarch64", target_arch = "x86_64")
))]
fn enable_platform_sandbox() -> seccompiler::Result<bool> {
    use seccompiler::{
        BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
        SeccompRule,
    };
    use std::env::consts::ARCH;

    // Some syscall numbers aren't yet defined in the libc crate for aarch64. Manually override
    // them here where necessary until upstream contains all the syscalls we need.
    //
    // The values are sourced from
    // https://android.googlesource.com/platform/bionic/+/0339184/libc/kernel/uapi/asm-generic/unistd.h
    #[cfg(target_arch = "x86_64")]
    const SYS_LSEEK: libc::c_long = libc::SYS_lseek;
    #[cfg(target_arch = "aarch64")]
    const SYS_LSEEK: libc::c_long = 62;
    #[cfg(target_arch = "x86_64")]
    const SYS_MMAP: libc::c_long = libc::SYS_mmap;
    #[cfg(target_arch = "aarch64")]
    const SYS_MMAP: libc::c_long = 222;

    let filter: BpfProgram = SeccompFilter::new(
        vec![
            (libc::SYS_close, vec![]),
            (libc::SYS_epoll_pwait, vec![]),
            (
                libc::SYS_fcntl,
                vec![SeccompRule::new(vec![SeccompCondition::new(
                    1,
                    SeccompCmpArgLen::Dword,
                    SeccompCmpOp::Eq,
                    libc::F_DUPFD_CLOEXEC as u64,
                )?])?],
            ),
            (libc::SYS_getuid, vec![]),
            (libc::SYS_ioctl, vec![]),
            (SYS_LSEEK, vec![]),
            (
                SYS_MMAP,
                vec![
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        (libc::PROT_READ | libc::PROT_WRITE) as u64,
                    )?])?,
                    SeccompRule::new(vec![SeccompCondition::new(
                        2,
                        SeccompCmpArgLen::Dword,
                        SeccompCmpOp::Eq,
                        libc::PROT_NONE as u64,
                    )?])?,
                ],
            ),
            (libc::SYS_munmap, vec![]),
            (libc::SYS_prctl, vec![]),
            (libc::SYS_read, vec![]),
            (libc::SYS_write, vec![]),
            (libc::SYS_writev, vec![]),
        ]
        .into_iter()
        .collect(),
        SeccompAction::KillProcess,
        SeccompAction::Allow,
        // This should never panic due to conditional compilation
        ARCH.try_into().unwrap(),
    )?
    .try_into()?;

    seccompiler::apply_filter_all_threads(&filter)?;

    Ok(true)
}

#[cfg(not(all(
    target_os = "android",
    target_endian = "little",
    any(target_arch = "aarch64", target_arch = "x86_64")
)))]
fn enable_platform_sandbox() -> seccompiler::Result<bool> {
    Ok(false)
}
