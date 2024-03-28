// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

use std::{
    fs::File,
    io::{self, Error as IoError, Read, Write},
    os::fd::FromRawFd,
    sync::Arc,
};

use jni::{
    errors::Error as JniError,
    objects::{JClass, JObject, JValueGen},
    sys::{jint, jlong, jsize},
    Executor, JNIEnv,
};

#[no_mangle]
unsafe extern "system" fn Java_app_accrescent_ina_Patcher_patch(
    env: JNIEnv,
    _class: JClass,
    old_file_fd: jint,
    patch: JObject,
    new: JObject,
) -> jlong {
    // SAFETY: The caller guarantees that `old_file_fd` is an owned, open file descriptor
    let old_file = unsafe { File::from_raw_fd(old_file_fd) };

    let vm = match env.get_java_vm() {
        Ok(vm) => Arc::new(vm),
        Err(_) => return -1,
    };
    let patch_stream = InputStream::new(Executor::new(Arc::clone(&vm)), patch);
    let mut new_stream = OutputStream::new(Executor::new(vm), new);

    match crate::patch(old_file, patch_stream, &mut new_stream) {
        Ok(read) => read as jlong,
        Err(_) => -1,
    }
}

struct InputStream<'a> {
    executor: Executor,
    input_stream: JObject<'a>,
}

impl<'a> InputStream<'a> {
    fn new(executor: Executor, input_stream: JObject<'a>) -> Self {
        Self {
            executor,
            input_stream,
        }
    }
}

impl<'a> Read for InputStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.executor
            .with_attached(|env| {
                // A Java array's length is represented by a jsize, and jsize::MAX may be smaller
                // than buf.len(). Therefore, clamp the maximum size of the temporary buffer we
                // create to jsize::MAX.
                let java_buf_len: jsize = buf.len().try_into().unwrap_or(jsize::MAX);

                // Create a temporary Java buffer to read our bytes into
                let java_buf = env.new_byte_array(java_buf_len)?;

                // Read at most java_buf_len bytes from the Java InputStream into our Java byte
                // array
                //
                // https://docs.oracle.com/javase/8/docs/api/java/io/InputStream.html#read-byte:A-int-int-
                let read: jint = env
                    .call_method(
                        &self.input_stream,
                        "read",
                        "([BII)I",
                        &[
                            JValueGen::Object(&java_buf),
                            JValueGen::Int(0),
                            JValueGen::Int(java_buf_len),
                        ],
                    )?
                    .try_into()?;

                // Copy our Java byte array into buf
                env.get_byte_array_region(java_buf, 0, bytemuck::cast_slice_mut::<u8, i8>(buf))?;

                Ok(read)
            })
            // If `read` doesn't fit into a usize, then the InputStream API dictates it must be -1
            // and that the stream is at EOF. The equivalent in Rust's Read API is returning 0, so
            // map the value.
            .map(|read| read.try_into().unwrap_or(0))
            .map_err(|e: JniError| IoError::other(e))
    }
}

struct OutputStream<'a> {
    executor: Executor,
    output_stream: JObject<'a>,
}

impl<'a> OutputStream<'a> {
    fn new(executor: Executor, output_stream: JObject<'a>) -> Self {
        Self {
            executor,
            output_stream,
        }
    }
}

impl<'a> Write for OutputStream<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.executor
            .with_attached(|env| {
                // Write buf to the Java OutputStream
                //
                // https://docs.oracle.com/javase/8/docs/api/java/io/OutputStream.html#write-byte:A-
                let java_buf = env.byte_array_from_slice(buf)?;
                env.call_method(
                    &self.output_stream,
                    "write",
                    "([B)V",
                    &[JValueGen::Object(&java_buf)],
                )?;
                Ok(buf.len())
            })
            .map_err(|e: JniError| IoError::other(e))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.executor
            .with_attached(|env| {
                // Flush the Java OutputStream
                //
                // https://docs.oracle.com/javase/8/docs/api/java/io/OutputStream.html#flush--
                env.call_method(&self.output_stream, "flush", "()V", &[])?;
                Ok(())
            })
            .map_err(|e: JniError| IoError::other(e))
    }
}

#[cfg(feature = "sandbox")]
#[no_mangle]
extern "system" fn Java_app_accrescent_ina_Patcher_enableSandbox(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    match crate::sandbox::enable_for_patching() {
        Ok(enabled) => jint::from(enabled),
        Err(_) => -1,
    }
}
