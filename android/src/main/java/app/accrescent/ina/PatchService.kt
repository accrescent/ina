// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

package app.accrescent.ina

import android.app.Service
import android.content.Intent
import android.os.IBinder

public class PatchService : Service() {
    override fun onBind(intent: Intent): IBinder? {
        return null
    }
}
