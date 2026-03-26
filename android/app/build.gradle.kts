import org.gradle.api.tasks.Exec
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

val repoRootDir = rootDir.parentFile
val coreffiDir = repoRootDir.resolve("coreffi")
val coreffiUdlFile = coreffiDir.resolve("src/synap.udl")
val coreffiTomlFile = coreffiDir.resolve("uniffi.toml")
val xtaskManifestFile = repoRootDir.resolve("xtask/Cargo.toml")
val generatedCoreffiBindingsDir = layout.buildDirectory.dir("generated/source/uniffi/coreffi/kotlin").get().asFile
val cargoPluginTargets = listOf("arm", "arm64", "x86", "x86_64")
val androidLocalProperties = Properties().apply {
    val localPropertiesFile = rootDir.resolve("local.properties")
    if (localPropertiesFile.isFile) {
        localPropertiesFile.inputStream().use(::load)
    }
}
val releaseSigningPropertiesFile = rootDir.resolve("keystore.properties")
val releaseSigningProperties = Properties().apply {
    if (releaseSigningPropertiesFile.isFile) {
        releaseSigningPropertiesFile.inputStream().use(::load)
    }
}

fun Properties.hasNonBlank(key: String): Boolean =
    getProperty(key)?.isNotBlank() == true

fun resolveConfigFile(path: String): File =
    File(path).let { if (it.isAbsolute) it else rootDir.resolve(path) }

fun latestChildDir(parent: File): File? =
    parent.listFiles()
        ?.filter { it.isDirectory() }
        ?.sortedByDescending { it.name }
        ?.firstOrNull()

fun resolveAndroidSdkDir(): File? {
    val configured = androidLocalProperties.getProperty("sdk.dir")
        ?: System.getenv("ANDROID_HOME")
        ?: System.getenv("ANDROID_SDK_ROOT")
    return configured?.let(::File)?.takeIf { it.exists() }
}

fun resolveAndroidNdkDir(): File? {
    val configured = androidLocalProperties.getProperty("ndk.dir")
        ?: System.getenv("ANDROID_NDK_HOME")
        ?: System.getenv("ANDROID_NDK_ROOT")
    configured
        ?.let(::File)
        ?.takeIf { it.exists() }
        ?.let { return it }

    val sdkDir = resolveAndroidSdkDir() ?: return null
    latestChildDir(sdkDir.resolve("ndk"))?.let { return it }
    return sdkDir.resolve("ndk-bundle").takeIf { it.exists() }
}

val resolvedAndroidNdkDir = resolveAndroidNdkDir()
val resolvedAndroidNdkVersion = resolvedAndroidNdkDir
    ?.takeIf { it.parentFile?.name == "ndk" }
    ?.name

fun resolveCargoHome(): File? {
    val configured = androidLocalProperties.getProperty("cargo.home")
        ?: System.getenv("CARGO_HOME")
    configured
        ?.let(::File)
        ?.takeIf { it.exists() }
        ?.let { return it }

    val homeDir = System.getProperty("user.home")?.let(::File)
    return listOfNotNull(
        File("/opt/cargo").takeIf { it.exists() },
        homeDir?.resolve(".cargo")?.takeIf { it.exists() },
    ).firstOrNull()
}

fun resolveRustupHome(): File? {
    val configured = androidLocalProperties.getProperty("rustup.home")
        ?: System.getenv("RUSTUP_HOME")
    configured
        ?.let(::File)
        ?.takeIf { it.exists() }
        ?.let { return it }

    val homeDir = System.getProperty("user.home")?.let(::File)
    return listOfNotNull(
        File("/opt/rustup").takeIf { it.exists() },
        homeDir?.resolve(".rustup")?.takeIf { it.exists() },
    ).firstOrNull()
}

fun resolveExecutable(name: String): File? {
    val pathCandidates = System.getenv("PATH")
        .orEmpty()
        .split(File.pathSeparatorChar)
        .filter(String::isNotBlank)
        .map { File(it, name) }

    val cargoHome = resolveCargoHome()
    val homeDir = System.getProperty("user.home")?.let(::File)
    val extraCandidates = listOfNotNull(
        cargoHome?.resolve("bin/$name"),
        homeDir?.resolve(".cargo/bin/$name"),
        File("/opt/cargo/bin/$name"),
        File("/usr/local/bin/$name"),
        File("/usr/bin/$name"),
    )

    return (pathCandidates + extraCandidates)
        .firstOrNull { it.isFile() && it.canExecute() }
}

fun withPrependedPath(vararg dirs: File): String =
    (dirs.map { it.absolutePath } + System.getenv("PATH").orEmpty())
        .filter(String::isNotBlank)
        .distinct()
        .joinToString(File.pathSeparator)

fun resolveRustupToolchain(rustup: File?, rustupHome: File?, cargoHome: File?): String {
    System.getenv("RUSTUP_TOOLCHAIN")
        ?.takeIf(String::isNotBlank)
        ?.let { return it }

    androidLocalProperties.getProperty("rustup.toolchain")
        ?.takeIf(String::isNotBlank)
        ?.let { return it }

    if (rustup != null) {
        runCatching {
            val process = ProcessBuilder(rustup.absolutePath, "show", "active-toolchain")
                .directory(rootDir)
                .apply {
                    if (rustupHome != null) {
                        environment()["RUSTUP_HOME"] = rustupHome.absolutePath
                    }
                    if (cargoHome != null) {
                        environment()["CARGO_HOME"] = cargoHome.absolutePath
                    }
                }
                .start()
            val output = process.inputStream.bufferedReader().use { it.readText() }.trim()
            process.waitFor()
            output
                .lineSequence()
                .firstOrNull()
                ?.substringBefore(' ')
                ?.takeIf(String::isNotBlank)
        }.getOrNull()?.let { return it }
    }

    val installedToolchains = rustupHome
        ?.resolve("toolchains")
        ?.listFiles()
        ?.filter { it.isDirectory() }
        ?.map { it.name }
        ?.sorted()
        .orEmpty()

    return installedToolchains.firstOrNull { it.startsWith("stable") }
        ?: installedToolchains.firstOrNull { it.startsWith("nightly") }
        ?: installedToolchains.firstOrNull()
        ?: "stable"
}

fun resolveRustupChannel(toolchain: String): String =
    toolchain
        .removePrefix("+")
        .substringBefore('-')
        .ifBlank { "stable" }

fun resolveToolchainBinary(rustupHome: File?, toolchain: String, binary: String): File? =
    rustupHome
        ?.resolve("toolchains")
        ?.resolve(toolchain)
        ?.resolve("bin")
        ?.resolve(binary)
        ?.takeIf { it.isFile() && it.canExecute() }

fun shellQuote(value: String): String = "'${value.replace("'", "'\"'\"'")}'"

val releaseKeystoreFile = releaseSigningProperties.getProperty("storeFile")
    ?.takeIf(String::isNotBlank)
    ?.let(::resolveConfigFile)
val hasReleaseSigning = releaseKeystoreFile?.isFile == true &&
    releaseSigningProperties.hasNonBlank("storePassword") &&
    releaseSigningProperties.hasNonBlank("keyAlias") &&
    releaseSigningProperties.hasNonBlank("keyPassword")

android {
    namespace = "com.fuwaki.synap"
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
        applicationId = "com.fuwaki.synap"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"

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
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
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

val rustup = resolveExecutable("rustup")
val cargoHome = resolveCargoHome()
val rustupHome = resolveRustupHome()
val cargoToolchain = resolveRustupToolchain(rustup, rustupHome, cargoHome)
val cargoChannel = resolveRustupChannel(cargoToolchain)
val toolchainCargoExecutable = resolveToolchainBinary(rustupHome, cargoToolchain, "cargo")
val toolchainRustcExecutable = resolveToolchainBinary(rustupHome, cargoToolchain, "rustc")
val cargoExecutable = toolchainCargoExecutable ?: resolveExecutable("cargo")
val rustcExecutable = toolchainRustcExecutable ?: resolveExecutable("rustc")
val rustToolBinDirs = listOfNotNull(
    toolchainCargoExecutable?.parentFile,
    cargoExecutable?.parentFile,
    rustcExecutable?.parentFile,
    rustup?.parentFile,
).distinct()
val cargoProfile = if (gradle.startParameter.taskNames.any { it.contains("release", ignoreCase = true) }) {
    "release"
} else {
    "debug"
}
val rustWrapperDir = layout.buildDirectory.dir("generated/rust-wrapper").get().asFile
val cargoWrapper = rustWrapperDir.resolve("cargo")
val rustcWrapper = rustWrapperDir.resolve("rustc")

fun writeRustWrapper(
    scriptFile: File,
    executable: File,
    exportedRustc: File? = null,
    extraPathDirs: List<File> = emptyList(),
) {
    val pathValue = withPrependedPath(*extraPathDirs.toTypedArray())
    val lines = buildList {
        add("#!/usr/bin/env sh")
        add("set -eu")
        add("export PATH=${shellQuote(pathValue)}")
        if (cargoHome != null) {
            add("export CARGO_HOME=${shellQuote(cargoHome.absolutePath)}")
        }
        if (rustupHome != null) {
            add("export RUSTUP_HOME=${shellQuote(rustupHome.absolutePath)}")
        }
        add("export RUSTUP_TOOLCHAIN=${shellQuote(cargoToolchain)}")
        if (exportedRustc != null) {
            add("export RUSTC=${shellQuote(exportedRustc.absolutePath)}")
        }
        add("exec ${shellQuote(executable.absolutePath)} \"$@\"")
    }

    scriptFile.parentFile.mkdirs()
    scriptFile.writeText(lines.joinToString("\n", postfix = "\n"))
    scriptFile.setExecutable(true)
}

if (cargoExecutable != null) {
    writeRustWrapper(
        scriptFile = cargoWrapper,
        executable = cargoExecutable,
        exportedRustc = rustcExecutable,
        extraPathDirs = rustToolBinDirs,
    )
}
if (rustcExecutable != null) {
    writeRustWrapper(
        scriptFile = rustcWrapper,
        executable = rustcExecutable,
        extraPathDirs = rustToolBinDirs,
    )
}

cargo {
    module = coreffiDir.absolutePath
    libname = "uniffi_synap_coreffi"
    targets = cargoPluginTargets
    profile = cargoProfile
    targetDirectory = repoRootDir.resolve("target").absolutePath
    apiLevel = 24
    prebuiltToolchains = true

    if (cargoWrapper.exists()) {
        cargoCommand = cargoWrapper.absolutePath
    } else if (cargoExecutable != null) {
        cargoCommand = cargoExecutable.absolutePath
    }
    if (rustcWrapper.exists()) {
        rustcCommand = rustcWrapper.absolutePath
    } else if (rustcExecutable != null) {
        rustcCommand = rustcExecutable.absolutePath
    }
    if (toolchainCargoExecutable == null) {
        rustupChannel = cargoChannel
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

val generateCoreffiBindings by tasks.registering(Exec::class) {
    group = "build"
    description = "Generate Kotlin UniFFI bindings for synap-coreffi via cargo xtask."

    workingDir = repoRootDir
    inputs.file(coreffiUdlFile)
    inputs.file(coreffiTomlFile)
    inputs.file(xtaskManifestFile)
    inputs.dir(repoRootDir.resolve("xtask/src"))
    outputs.dir(generatedCoreffiBindingsDir)

    doFirst {
        val cargo = toolchainCargoExecutable
            ?: resolveExecutable("cargo")
            ?: throw GradleException("cargo not found. Install Rust and ensure cargo is available to Gradle.")

        delete(generatedCoreffiBindingsDir)
        generatedCoreffiBindingsDir.mkdirs()

        environment("PATH", withPrependedPath(cargo.parentFile))
        if (cargoHome != null) {
            environment("CARGO_HOME", cargoHome.absolutePath)
        }
        if (rustupHome != null) {
            environment("RUSTUP_HOME", rustupHome.absolutePath)
        }
        if (toolchainCargoExecutable == null) {
            environment("RUSTUP_TOOLCHAIN", cargoToolchain)
        }
        environment("CARGO", cargo.absolutePath)
        commandLine(
            cargo.absolutePath,
            "run",
            "--quiet",
            "-p",
            "xtask",
            "--",
            "gen-uniffi-kotlin",
            "--udl",
            coreffiUdlFile.absolutePath,
            "--config",
            coreffiTomlFile.absolutePath,
            "--out-dir",
            generatedCoreffiBindingsDir.absolutePath,
        )
    }
}

tasks.withType(Exec::class.java).configureEach {
    if (name == "cargoBuild" || name.startsWith("cargoBuild")) {
        if (rustToolBinDirs.isNotEmpty()) {
            environment("PATH", withPrependedPath(*rustToolBinDirs.toTypedArray()))
        }
        if (cargoHome != null) {
            environment("CARGO_HOME", cargoHome.absolutePath)
        }
        if (rustupHome != null) {
            environment("RUSTUP_HOME", rustupHome.absolutePath)
        }
        if (toolchainCargoExecutable == null) {
            environment("RUSTUP_TOOLCHAIN", cargoToolchain)
        }
        cargoExecutable?.let { environment("CARGO", it.absolutePath) }
        rustcExecutable?.let { environment("RUSTC", it.absolutePath) }
    }
}

tasks.named("preBuild").configure {
    dependsOn(generateCoreffiBindings)
    dependsOn("cargoBuild")
}
