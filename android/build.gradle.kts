// Copyright 2024 Logan Magee
//
// SPDX-License-Identifier: MPL-2.0

import org.apache.tools.ant.taskdefs.condition.Os
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_MAC
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_UNIX
import org.apache.tools.ant.taskdefs.condition.Os.FAMILY_WINDOWS
import org.jetbrains.dokka.gradle.DokkaTask
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.jetbrains.kotlin.android)
    alias(libs.plugins.android.library)
    alias(libs.plugins.dokka)
}

val inaMinSdk = 29

android {
    namespace = "app.accrescent.ina"
    compileSdk = 36

    buildToolsVersion = "36.0.0"
    ndkVersion = "27.3.13750724"

    defaultConfig {
        minSdk = inaMinSdk

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    testOptions {
        managedDevices {
            localDevices {
                create("nexusOneApi34") {
                    device = "Nexus One"
                    apiLevel = 34
                    systemImageSource = "aosp"
                }
            }
        }
    }
}

kotlin {
    jvmToolchain(21)

    compilerOptions {
        jvmTarget = JvmTarget.JVM_21
    }

    explicitApi()
}

dependencies {
    androidTestImplementation(libs.androidx.test.core)
    androidTestImplementation(libs.androidx.test.rules)
    androidTestImplementation(libs.androidx.test.runner)
    testImplementation(libs.junit)
}

tasks.register("buildJniLibs") {
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
    val x8664CcPath = "$toolchainDir/x86_64-linux-android$api-clang"

    doFirst {
        exec {
            environment("AR", "$toolchainDir/llvm-ar")
            environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", aarch64CcPath)
            environment("CC_aarch64-linux-android", aarch64CcPath)

            commandLine(
                "cargo",
                "build",
                "-p",
                "ina",
                "--no-default-features",
                "--features",
                "java-ffi,patch,sandbox",
                "--target",
                "aarch64-linux-android",
                "--profile",
                "cdylib-release",
            )
        }
        exec {
            environment("AR", "$toolchainDir/llvm-ar")
            environment("CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER", x8664CcPath)
            environment("CC_x86_64-linux-android", x8664CcPath)

            commandLine(
                "cargo",
                "build",
                "-p",
                "ina",
                "--no-default-features",
                "--features",
                "java-ffi,patch,sandbox",
                "--target",
                "x86_64-linux-android",
                "--profile",
                "cdylib-release",
            )
        }
    }

    doLast {
        copy {
            from("$rootDir/target/aarch64-linux-android/cdylib-release/libina.so")
            into("$projectDir/src/main/jniLibs/arm64-v8a")
        }
        copy {
            from("$rootDir/target/x86_64-linux-android/cdylib-release/libina.so")
            into("$projectDir/src/main/jniLibs/x86_64")
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

tasks.withType<DokkaTask>().configureEach {
    failOnWarning.set(true)

    dokkaSourceSets {
        configureEach {
            reportUndocumented.set(true)
            suppressInheritedMembers.set(true)
        }
    }
}
