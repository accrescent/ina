// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

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
 * The result of a patch operations
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
    val oldFileFd = ParcelFileDescriptor.open(oldFile, MODE_READ_ONLY)

    // Adapt the "patch" stream to a ParcelFileDescriptor that can later be opened as an InputStream
    val patchPipeFds = ParcelFileDescriptor.createPipe()
    val patchReaderFd = patchPipeFds[0]
    val patchWriterFd = patchPipeFds[1]
    thread {
        AutoCloseOutputStream(patchWriterFd).use { out ->
            patch().use { it.copyTo(out) }
        }
    }

    // Adapt the "new" stream to a ParcelFileDescriptor that can later be opened as an OutputStream
    val newPipeFds = ParcelFileDescriptor.createPipe()
    val newReaderFd = newPipeFds[0]
    val newWriterFd = newPipeFds[1]
    thread {
        AutoCloseInputStream(newReaderFd).use { inStream ->
            new().use { inStream.copyTo(it) }
        }
    }

    val message = Message.obtain(null, MSG_PATCH).apply {
        data = Bundle().apply {
            putParcelable("oldFileFd", oldFileFd)
            putParcelable("patchFd", patchReaderFd)
            putParcelable("outFd", newWriterFd)
        }
        replyTo = Messenger(ResponseHandler(onComplete))
    }
    messenger.send(message)
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
