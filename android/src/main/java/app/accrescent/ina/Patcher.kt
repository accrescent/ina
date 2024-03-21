// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

package app.accrescent.ina

import java.io.IOException
import java.io.InputStream
import java.io.OutputStream

internal class Patcher {
    companion object {
        init {
            System.loadLibrary("ina")
        }

        /**
         * Patches an old file given an Ina patch stream
         *
         * # Safety
         *
         * [oldFileFd] must be an owned, open file descriptor
         */
        @JvmStatic
        @Throws(IOException::class)
        external fun patch(oldFileFd: Int, patch: InputStream, out: OutputStream): Long
    }
}
