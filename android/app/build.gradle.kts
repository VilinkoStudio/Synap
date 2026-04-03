import java.io.File
import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.ksp)
    alias(libs.plugins.hilt)
    id("org.mozilla.rust-android-gradle.rust-android") version "0.9.6"
}

val generatedCoreffiBindingsDir = layout.buildDirectory.dir("generated/source/uniffi/coreffi/kotlin").get().asFile

val androidLocalProperties = Properties().apply {
    rootDir.resolve("local.properties").takeIf { it.isFile }?.inputStream()?.use(::load)
}
val releaseSigningProperties = Properties().apply {
    rootDir.resolve("keystore.properties").takeIf { it.isFile }?.inputStream()?.use(::load)
}

fun Properties.hasNonBlank(key: String): Boolean =
    getProperty(key)?.isNotBlank() == true

fun resolveConfigFile(path: String): File =
    File(path).let { if (it.isAbsolute) it else rootDir.resolve(path) }

fun latestChildDir(parent: File): File? =
    parent.listFiles()?.filter(File::isDirectory)?.maxByOrNull { it.name }

fun runGitCommand(repoRoot: File, vararg args: String): String? {
    val output = runCatching {
        ProcessBuilder(listOf("git", *args))
            .directory(repoRoot)
            .redirectErrorStream(true)
            .start()
            .let { process ->
                val text = process.inputStream.bufferedReader().use { it.readText().trim() }
                if (process.waitFor() == 0) {
                    text.ifBlank { null }
                } else {
                    null
                }
            }
    }.getOrNull()

    return output
}

val repoRootDir = rootDir.parentFile ?: rootDir
val gitShortCommit = runGitCommand(repoRootDir, "rev-parse", "--short=12", "HEAD") ?: "unknown"
val gitBranch = runGitCommand(repoRootDir, "rev-parse", "--abbrev-ref", "HEAD")
    ?.let { branch -> if (branch == "HEAD") "detached" else branch }
    ?: "unknown"
val gitTag = runGitCommand(repoRootDir, "describe", "--tags", "--exact-match", "HEAD")
val resolvedVersionName = gitTag?.let { "$it ($gitShortCommit)" } ?: "$gitBranch ($gitShortCommit)"

fun resolveAndroidSdkDir(): File? {
    val configured = androidLocalProperties.getProperty("sdk.dir")
        ?: System.getenv("ANDROID_HOME")
        ?: System.getenv("ANDROID_SDK_ROOT")
    return configured?.let(::File)?.takeIf(File::exists)
}

fun resolveAndroidNdkDir(): File? {
    val configured = androidLocalProperties.getProperty("ndk.dir")
        ?: System.getenv("ANDROID_NDK_HOME")
        ?: System.getenv("ANDROID_NDK_ROOT")
    configured?.let(::File)?.takeIf(File::exists)?.let { return it }

    val sdkDir = resolveAndroidSdkDir() ?: return null
    return latestChildDir(sdkDir.resolve("ndk"))
        ?: sdkDir.resolve("ndk-bundle").takeIf(File::exists)
}

val resolvedAndroidNdkVersion = resolveAndroidNdkDir()
    ?.takeIf { it.parentFile?.name == "ndk" }
    ?.name

val releaseKeystoreFile = releaseSigningProperties.getProperty("storeFile")
    ?.takeIf(String::isNotBlank)
    ?.let(::resolveConfigFile)

val hasReleaseSigning = releaseKeystoreFile?.isFile == true &&
    releaseSigningProperties.hasNonBlank("storePassword") &&
    releaseSigningProperties.hasNonBlank("keyAlias") &&
    releaseSigningProperties.hasNonBlank("keyPassword")

android {
    namespace = "com.synap.app"
    compileSdk = 36
    if (resolvedAndroidNdkVersion != null) {
        ndkVersion = resolvedAndroidNdkVersion
    }

    signingConfigs {
        if (hasReleaseSigning) {
            create("release") {
                storeFile = requireNotNull(releaseKeystoreFile)
                storePassword = releaseSigningProperties.getProperty("storePassword")
                keyAlias = releaseSigningProperties.getProperty("keyAlias")
                keyPassword = releaseSigningProperties.getProperty("keyPassword")
            }
        }
    }

    defaultConfig {
        applicationId = "com.synap.app"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = resolvedVersionName
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64"))
        }
    }

    buildTypes {
        debug {
            isDebuggable = true
        }
        release {
            if (hasReleaseSigning) {
                signingConfig = signingConfigs.getByName("release")
            }
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    kotlinOptions {
        jvmTarget = "11"
    }
    buildFeatures {
        compose = true
    }

    sourceSets {
        getByName("main") {
            java.srcDir(generatedCoreffiBindingsDir)
        }
    }

    packaging {
        resources {
            excludes += "META-INF/AL2.0"
            excludes += "META-INF/LGPL2.1"
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.18.1@aar")
    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation("androidx.hilt:hilt-navigation-compose:1.3.0")
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation("androidx.compose.animation:animation")
    implementation("androidx.compose.material:material-icons-extended")
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)
}

apply(from = rootProject.file("gradle/rust-uniffi.gradle.kts"))
