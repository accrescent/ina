// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: LicenseRef-Proprietary

import org.apache.tools.ant.taskdefs.condition.Os
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_MAC
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_UNIX
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_WINDOWS

plugins {
    alias(libs.plugins.jetbrains.kotlin.android)
    alias(libs.plugins.android.library)
}

val inaMinSdk = 29

android {
    namespace = "app.accrescent.ina"
    compileSdk = 34

    buildToolsVersion = "34.0.0"
    ndkVersion = "26.2.11394342"

    defaultConfig {
        minSdk = inaMinSdk
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

kotlin {
    explicitApi()
}

tasks.register<Exec>("buildJniLibs") {
    // Obtain the host tag for the current system so we can find the proper toolchain directory
    //
    // https://developer.android.com/ndk/guides/other_build_systems#overview
    val hostTag = when {
        Os.isFamily(FAMILY_MAC) -> "darwin-x86_64"
        Os.isFamily(FAMILY_UNIX) -> "linux-x86_64"
        Os.isFamily(FAMILY_WINDOWS) -> "windows-x86_64"
        else -> throw TaskExecutionException(this, Exception("unsupported host platform"))
    }
    val toolchainDir = "${android.ndkDirectory}/toolchains/llvm/prebuilt/$hostTag/bin"
    val api = inaMinSdk

    val aarch64CcPath = "$toolchainDir/aarch64-linux-android$api-clang"
    environment("AR", "$toolchainDir/llvm-ar")
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", aarch64CcPath)
    environment("CC_aarch64-linux-android", aarch64CcPath)

    commandLine("cargo", "build", "-p", "ina", "--target", "aarch64-linux-android", "--release")

    doLast {
        copy {
            from("$rootDir/target/aarch64-linux-android/release/libina.so")
            into("$projectDir/src/main/jniLibs/arm64-v8a")
        }
    }
}

tasks.register<Exec>("cargoClean") {
    commandLine("cargo", "clean")
}

tasks.preBuild {
    dependsOn(tasks.getByName("buildJniLibs"))
}

tasks.clean {
    dependsOn(tasks.getByName("cargoClean"))

    delete(fileTree("$projectDir/src/main/jniLibs"))
}
