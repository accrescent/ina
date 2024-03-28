// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

package app.accrescent.ina

import android.content.Context
import android.content.Intent
import android.os.ConditionVariable
import android.os.Messenger
import android.os.ParcelFileDescriptor
import android.os.ParcelFileDescriptor.MODE_READ_ONLY
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.ServiceTestRule
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest

private const val OLD_FILE_NAME = "gcc-13.1.1"
private const val NEW_FILE_NAME = "gcc-13.2.1"
private const val PATCH_FILE_NAME = "gcc-13.1.1-13.2.1.ina"

private const val EXPECTED_READ: Long = 1951728
private const val EXPECTED_NEW_HASH =
    "0421e7f96812b62d4779f3ed990cca16bce7153af2c3f99497705048a256b55b"

class PatchServiceTest {
    @get:Rule
    val serviceRule = ServiceTestRule()

    @Test
    fun patchSucceeds() {
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
        val newFile = File(context.cacheDir, NEW_FILE_NAME)

        val receivedResponse = ConditionVariable(false)

        // Use the overload which adapts both streams to ParcelFileDescriptors to test the most
        // complex code path
        submitPatchRequest(
            messenger,
            oldFile,
            { testContext.assets.open(PATCH_FILE_NAME) },
            { newFile.outputStream() },
        ) { res ->
            when (res) {
                is PatchResult.Ok -> assertEquals(res.bytesWritten, EXPECTED_READ)
                PatchResult.Error -> fail("Patch result is an error")
            }

            val newHash = MessageDigest.getInstance("SHA-256")
                .digest(File(context.cacheDir, NEW_FILE_NAME).readBytes())
                .joinToString("") { "%02x".format(it) }
            assertEquals(EXPECTED_NEW_HASH, newHash)

            receivedResponse.open()
        }

        receivedResponse.block()
    }

    @Test
    fun patchFailsOnException() {
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
        val newFile = File(context.cacheDir, NEW_FILE_NAME)

        val receivedResponse = ConditionVariable(false)

        submitPatchRequest(
            messenger,
            oldFile,
            { testContext.assets.open(PATCH_FILE_NAME) },
            // The new file is read-only, which should cause writes to throw an IOException
            ParcelFileDescriptor.open(newFile, MODE_READ_ONLY),
        ) { res ->
            assertTrue(res is PatchResult.Error)

            receivedResponse.open()
        }

        receivedResponse.block()
    }
}
