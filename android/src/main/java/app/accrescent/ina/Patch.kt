// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

package app.accrescent.ina

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.Message
import android.os.Messenger
import android.os.ParcelFileDescriptor
import android.os.ParcelFileDescriptor.AutoCloseInputStream
import android.os.ParcelFileDescriptor.AutoCloseOutputStream
import android.os.ParcelFileDescriptor.MODE_READ_ONLY
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import kotlin.concurrent.thread

/**
 * The result of a patch operation
 */
public sealed class PatchResult {
    /**
     * The result of a successful patch operation
     *
     * @property bytesWritten the number of bytes written to the new blob
     */
    public data class Ok(val bytesWritten: Long) : PatchResult()

    /**
     * An error representing a failed patch operation
     */
    public data object Error : PatchResult()
}

/**
 * Submits a request to reconstruct a new blob from an old blob and a patch
 *
 * This method returns immediately, calling [onComplete] when patching is complete. If called
 * multiple times in sequence, each patch request is put in a queue and run sequentially.
 *
 * Requests are run in a separate service process and sandboxed for increased security. For more
 * details, see this software's external documentation.
 *
 * **Note**: This overload of this function spawns two threads to convert [patch] and [new] to
 * [ParcelFileDescriptor]s. If you can access either your patch input, new destination, or both as
 * `ParcelFileDescriptor`s directly, prefer passing those to other overloads of this function for
 * better performance.
 *
 * @param messenger a [Messenger] associated with the bound service
 * @param oldFile the old file
 * @param patch a lambda which returns the patch as an [InputStream]
 * @param new a lambda which returns the new blob destination as an [OutputStream]
 * @param onComplete a lambda which is called when patching completes
 */
public fun submitPatchRequest(
    messenger: Messenger,
    oldFile: File,
    patch: () -> InputStream,
    new: () -> OutputStream,
    onComplete: (PatchResult) -> Unit,
) {
    submitPatchRequestImpl(messenger, oldFile, patch.toReaderFd(), new.toWriterFd(), onComplete)
}

/**
 * Submits a request to reconstruct a new blob from an old blob and a patch
 *
 * This method returns immediately, calling [onComplete] when patching is complete. If called
 * multiple times in sequence, each patch request is put in a queue and run sequentially.
 *
 * Requests are run in a separate service process and sandboxed for increased security. For more
 * details, see this software's external documentation.
 *
 * **Note**: This overload of this function spawns a thread to convert [new] to a
 * [ParcelFileDescriptor]. If you can access your new destination as a `ParcelFileDescriptor`
 * directly, prefer passing it to the respective overload of this function for better performance.
 *
 * @param messenger a [Messenger] associated with the bound service
 * @param oldFile the old file
 * @param patch the patch to read from
 * @param new the new blob destination
 * @param onComplete a lambda which is called when patching completes
 */
public fun submitPatchRequest(
    messenger: Messenger,
    oldFile: File,
    patch: ParcelFileDescriptor,
    new: () -> OutputStream,
    onComplete: (PatchResult) -> Unit,
) {
    submitPatchRequestImpl(messenger, oldFile, patch, new.toWriterFd(), onComplete)
}

/**
 * Submits a request to reconstruct a new blob from an old blob and a patch
 *
 * This method returns immediately, calling [onComplete] when patching is complete. If called
 * multiple times in sequence, each patch request is put in a queue and run sequentially.
 *
 * Requests are run in a separate service process and sandboxed for increased security. For more
 * details, see this software's external documentation.
 *
 * **Note**: This overload of this function spawns a thread to convert [patch] to a
 * [ParcelFileDescriptor]. If you can access your patch as a `ParcelFileDescriptor` directly, prefer
 * passing it to the respective overload of this function for better performance.
 *
 * @param messenger a [Messenger] associated with the bound service
 * @param oldFile the old file
 * @param patch a lambda which returns the patch as an [InputStream]
 * @param new the new blob destination
 * @param onComplete a lambda which is called when patching completes
 */
public fun submitPatchRequest(
    messenger: Messenger,
    oldFile: File,
    patch: () -> InputStream,
    new: ParcelFileDescriptor,
    onComplete: (PatchResult) -> Unit,
) {
    submitPatchRequestImpl(messenger, oldFile, patch.toReaderFd(), new, onComplete)
}

/**
 * Submits a request to reconstruct a new blob from an old blob and a patch
 *
 * This method returns immediately, calling [onComplete] when patching is complete. If called
 * multiple times in sequence, each patch request is put in a queue and run sequentially.
 *
 * Requests are run in a separate service process and sandboxed for increased security. For more
 * details, see this software's external documentation.
 *
 * **Note**: This overload of this function should be preferred whenever the patch and new
 * destination are accessible as [ParcelFileDescriptor]s for better performance.
 *
 * @param messenger a [Messenger] associated with the bound service
 * @param oldFile the old file
 * @param patch the patch to read from
 * @param new the new blob destination
 * @param onComplete a lambda which is called when patching completes
 */
public fun submitPatchRequest(
    messenger: Messenger,
    oldFile: File,
    patch: ParcelFileDescriptor,
    new: ParcelFileDescriptor,
    onComplete: (PatchResult) -> Unit,
) {
    submitPatchRequestImpl(messenger, oldFile, patch, new, onComplete)
}

private fun submitPatchRequestImpl(
    messenger: Messenger,
    oldFile: File,
    patchFd: ParcelFileDescriptor,
    newFd: ParcelFileDescriptor,
    onComplete: (PatchResult) -> Unit,
) {
    val oldFileFd = ParcelFileDescriptor.open(oldFile, MODE_READ_ONLY)

    val message = Message.obtain(null, MSG_PATCH).apply {
        data = Bundle().apply {
            putParcelable("oldFileFd", oldFileFd)
            putParcelable("patchFd", patchFd)
            putParcelable("newFd", newFd)
        }
        replyTo = Messenger(ResponseHandler(onComplete))
    }
    messenger.send(message)
}

/**
 * Adapts the stream to a [ParcelFileDescriptor] that can later be reopened as an [InputStream]
 */
private fun (() -> InputStream).toReaderFd(): ParcelFileDescriptor {
    val pipeFds = ParcelFileDescriptor.createPipe()
    val readerFd = pipeFds[0]
    val writerFd = pipeFds[1]
    thread {
        AutoCloseOutputStream(writerFd).use { out ->
            this().use { it.copyTo(out) }
        }
    }

    return readerFd
}

/**
 * Adapts the stream to a [ParcelFileDescriptor] that can later be reopened as an [OutputStream]
 */
private fun (() -> OutputStream).toWriterFd(): ParcelFileDescriptor {
    val pipeFds = ParcelFileDescriptor.createPipe()
    val readerFd = pipeFds[0]
    val writerFd = pipeFds[1]
    thread {
        AutoCloseInputStream(readerFd).use { inStream ->
            this().use { inStream.copyTo(it) }
        }
    }

    return writerFd
}

private class ResponseHandler(
    private val onComplete: (PatchResult) -> Unit,
) : Handler(Looper.getMainLooper()) {
    override fun handleMessage(msg: Message) {
        when (msg.what) {
            RESP_PATCH_SUCCESS -> {
                val written = msg.data.getLong("bytesWritten")
                onComplete(PatchResult.Ok(written))
            }

            RESP_PATCH_FAILURE -> onComplete(PatchResult.Error)
            else -> super.handleMessage(msg)
        }
    }
}
