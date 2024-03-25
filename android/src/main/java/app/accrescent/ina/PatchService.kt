// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

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

/**
 * Message ID indicating the message body is a patch request
 */
public const val MSG_PATCH: Int = 1

/**
 * Message ID indicating the message body is a success response
 */
public const val RESP_PATCH_SUCCESS: Int = 2

/**
 * Message ID indicating the message body is a patch failure
 */
public const val RESP_PATCH_FAILURE: Int = 3

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
                    val outFd =
                        msg.data.getParcelableCompat("outFd", ParcelFileDescriptor::class.java)
                    val clientHandle = msg.replyTo

                    AutoCloseInputStream(patchFd).use { patch ->
                        AutoCloseOutputStream(outFd).use { out ->
                            val read = Patcher.patch(oldFileFd, patch, out)

                            val response = Message.obtain().apply {
                                if (read != -1L) {
                                    what = RESP_PATCH_SUCCESS
                                    data.putLong("read", read)
                                } else {
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
