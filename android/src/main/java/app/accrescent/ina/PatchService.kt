// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

package app.accrescent.ina

import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.Message
import android.os.Messenger
import android.os.ParcelFileDescriptor
import android.os.ParcelFileDescriptor.AutoCloseInputStream
import android.os.ParcelFileDescriptor.AutoCloseOutputStream
import android.util.Log
import java.io.IOException
import java.security.GeneralSecurityException

/**
 * Message ID indicating the message body is a patch request
 */
internal const val MSG_PATCH: Int = 1

/**
 * Message ID indicating the message body is a success response
 */
internal const val RESP_PATCH_SUCCESS: Int = 2

/**
 * Message ID indicating the message body is a patch failure
 */
internal const val RESP_PATCH_FAILURE: Int = 3

private const val TAG: String = "Ina"

/**
 * A service that patches blobs using Ina patch files
 *
 * The patching process is asynchronous and facilitated by the Android [Messenger] API.
 */
public class PatchService : Service() {
    private lateinit var messenger: Messenger

    internal class IncomingHandler : Handler(Looper.getMainLooper()) {
        override fun handleMessage(msg: Message) {
            when (msg.what) {
                MSG_PATCH -> {
                    val oldFileFd =
                        msg.data.getParcelableCompat("oldFileFd", ParcelFileDescriptor::class.java)
                            ?.detachFd() ?: return
                    val patchFd =
                        msg.data.getParcelableCompat("patchFd", ParcelFileDescriptor::class.java)
                    val newFd =
                        msg.data.getParcelableCompat("newFd", ParcelFileDescriptor::class.java)
                    val clientHandle = msg.replyTo

                    AutoCloseInputStream(patchFd).use { patch ->
                        AutoCloseOutputStream(newFd).use { new ->
                            val response = Message.obtain().apply {
                                try {
                                    val bytesWritten = Patcher.patch(oldFileFd, patch, new)

                                    if (bytesWritten != -1L) {
                                        what = RESP_PATCH_SUCCESS
                                        data.putLong("bytesWritten", bytesWritten)
                                    } else {
                                        what = RESP_PATCH_FAILURE
                                    }
                                } catch (e: IOException) {
                                    what = RESP_PATCH_FAILURE
                                }
                            }
                            clientHandle.send(response)
                        }
                    }
                }

                else -> super.handleMessage(msg)
            }
        }
    }

    /**
     * @suppress
     */
    override fun onBind(intent: Intent): IBinder {
        messenger = Messenger(IncomingHandler())
        return messenger.binder
    }

    /**
     * @suppress
     */
    override fun onCreate() {
        // Prevent the service from starting if sandbox initialization fails so we never process
        // untrusted data outside of a sandbox
        when (Patcher.enableSandbox()) {
            1 -> Log.i(TAG, "Successfully enabled seccomp sandbox")
            0 -> throw GeneralSecurityException("Seccomp sandbox unavailable. This should never happen.")
            -1 -> throw GeneralSecurityException("Seccomp sandbox initialization failed")
            else -> throw GeneralSecurityException("Unknown seccomp sandbox error occurred. This should never happen.")
        }
    }
}

private fun <T : Any> Bundle.getParcelableCompat(key: String, clazz: Class<T>): T? {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        getParcelable(key, clazz)
    } else {
        // This is the only getParcelable method available before Build.VERSION_CODES.TIRAMISU
        @Suppress("DEPRECATION")
        getParcelable(key)
    }
}
