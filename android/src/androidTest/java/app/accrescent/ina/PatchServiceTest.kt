// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

package app.accrescent.ina

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.os.ConditionVariable
import android.os.Handler
import android.os.Looper
import android.os.Message
import android.os.Messenger
import android.os.ParcelFileDescriptor
import android.os.ParcelFileDescriptor.AutoCloseOutputStream
import android.os.ParcelFileDescriptor.MODE_CREATE
import android.os.ParcelFileDescriptor.MODE_READ_ONLY
import android.os.ParcelFileDescriptor.MODE_TRUNCATE
import android.os.ParcelFileDescriptor.MODE_WRITE_ONLY
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.ServiceTestRule
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest
import kotlin.concurrent.thread

private const val OLD_FILE_NAME = "gcc-13.1.1"
private const val NEW_FILE_NAME = "gcc-13.2.1"
private const val PATCH_FILE_NAME = "gcc-13.1.1-13.2.1.ina"

private const val EXPECTED_READ: Long = 1951728
private const val EXPECTED_NEW_HASH =
    "0421e7f96812b62d4779f3ed990cca16bce7153af2c3f99497705048a256b55b"

class PatchServiceTest {
    @get:Rule
    val serviceRule = ServiceTestRule()

    internal class ResponseHandler(
        private val context: Context,
        private val receivedResponse: ConditionVariable,
    ) : Handler(Looper.getMainLooper()) {
        override fun handleMessage(msg: Message) {
            assertEquals(RESP_PATCH_SUCCESS, msg.what)

            val read = msg.data.getLong("read")
            assertEquals(EXPECTED_READ, read)

            val newHash = MessageDigest.getInstance("SHA-256")
                .digest(File(context.cacheDir, NEW_FILE_NAME).readBytes())
                .joinToString("") { "%02x".format(it) }
            assertEquals(EXPECTED_NEW_HASH, newHash)

            receivedResponse.open()
        }
    }

    @Test
    fun testPatch() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val testContext = InstrumentationRegistry.getInstrumentation().context
        val serviceIntent = Intent(context, PatchService::class.java)
        val messenger = Messenger(serviceRule.bindService(serviceIntent))

        // Copy old file from assets folder to internal storage so we can get a proper file
        // descriptor
        val oldFile = File(context.cacheDir, OLD_FILE_NAME)
        FileOutputStream(oldFile).use { oldFileInternal ->
            testContext.assets.open(OLD_FILE_NAME).use { oldFileAsset ->
                oldFileAsset.copyTo(oldFileInternal)
            }
        }
        val oldFileFd = ParcelFileDescriptor.open(oldFile, MODE_READ_ONLY).detachFd()

        val descriptors = ParcelFileDescriptor.createPipe()
        val readDesc = descriptors[0]
        val writeDesc = descriptors[1]
        thread(start = true) {
            testContext.assets.open(PATCH_FILE_NAME).use { patch ->
                AutoCloseOutputStream(writeDesc).use { out ->
                    patch.copyTo(out)
                }
            }
        }

        val outFd = ParcelFileDescriptor.open(
            File(context.cacheDir, NEW_FILE_NAME),
            MODE_CREATE or MODE_TRUNCATE or MODE_WRITE_ONLY,
        )

        val receivedResponse = ConditionVariable(false)

        val message = Message.obtain(null, MSG_PATCH)
        message.data = Bundle().apply {
            putInt("oldFileFd", oldFileFd)
            putParcelable("patchFd", readDesc)
            putParcelable("outFd", outFd)
        }
        message.replyTo = Messenger(ResponseHandler(context, receivedResponse))
        messenger.send(message)

        receivedResponse.block()
    }
}
