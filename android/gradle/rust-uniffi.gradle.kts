import org.gradle.api.tasks.Exec
import java.io.File
import java.util.Properties

val isWindows = System.getProperty("os.name").lowercase().contains("windows")

val repoRootDir = rootDir.parentFile
val coreffiDir = repoRootDir.resolve("coreffi")
val coreffiUdlFile = coreffiDir.resolve("src/synap.udl")
val coreffiTomlFile = coreffiDir.resolve("uniffi.toml")
val xtaskManifestFile = repoRootDir.resolve("xtask/Cargo.toml")
val generatedCoreffiBindingsDir = layout.buildDirectory.dir("generated/source/uniffi/coreffi/kotlin").get().asFile
val cargoPluginTargets = listOf("arm", "arm64", "x86", "x86_64")

val androidLocalProperties = Properties().apply {
    rootDir.resolve("local.properties").takeIf { it.isFile }?.inputStream()?.use(::load)
}

fun resolveCargoHome(): File? {
    val configured = androidLocalProperties.getProperty("cargo.home") ?: System.getenv("CARGO_HOME")
    configured?.let(::File)?.takeIf(File::exists)?.let { return it }

    val homeDir = System.getProperty("user.home")?.let(::File)
    return listOfNotNull(
        File("/opt/cargo").takeIf(File::exists),
        homeDir?.resolve(".cargo")?.takeIf(File::exists),
    ).firstOrNull()
}

fun resolveRustupHome(): File? {
    val configured = androidLocalProperties.getProperty("rustup.home") ?: System.getenv("RUSTUP_HOME")
    configured?.let(::File)?.takeIf(File::exists)?.let { return it }

    val homeDir = System.getProperty("user.home")?.let(::File)
    return listOfNotNull(
        File("/opt/rustup").takeIf(File::exists),
        homeDir?.resolve(".rustup")?.takeIf(File::exists),
    ).firstOrNull()
}

fun resolveExecutable(name: String): File? {
    val execName = if (isWindows) "$name.exe" else name
    val pathCandidates = System.getenv("PATH")
        .orEmpty()
        .split(File.pathSeparatorChar)
        .filter(String::isNotBlank)
        .map { File(it, execName) }

    val cargoHome = resolveCargoHome()
    val homeDir = System.getProperty("user.home")?.let(::File)
    val userProfile = System.getenv("USERPROFILE")?.let(::File)
    val extraCandidates = listOfNotNull(
        cargoHome?.resolve("bin/$execName"),
        homeDir?.resolve(".cargo/bin/$execName"),
        userProfile?.resolve(".cargo/bin/$execName"),
        File("/opt/cargo/bin/$execName"),
        File("/usr/local/bin/$execName"),
        File("/usr/bin/$execName"),
    )

    return (pathCandidates + extraCandidates).firstOrNull { it.isFile && it.canExecute() }
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
        ?.filter(File::isDirectory)
        ?.map { it.name }
        ?.sorted()
        .orEmpty()

    return installedToolchains.firstOrNull { it.startsWith("stable") }
        ?: installedToolchains.firstOrNull { it.startsWith("nightly") }
        ?: installedToolchains.firstOrNull()
        ?: "stable"
}

fun resolveRustupChannel(toolchain: String): String =
    toolchain.removePrefix("+").substringBefore('-').ifBlank { "stable" }

fun resolveToolchainBinary(rustupHome: File?, toolchain: String, binary: String): File? {
    val execName = if (isWindows) "$binary.exe" else binary
    return rustupHome
        ?.resolve("toolchains")
        ?.resolve(toolchain)
        ?.resolve("bin")
        ?.resolve(execName)
        ?.takeIf { it.isFile && it.canExecute() }
}

fun shellQuote(value: String): String = "'${value.replace("'", "'\"'\"'")}'"

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
val cargoWrapper = rustWrapperDir.resolve(if (isWindows) "cargo.bat" else "cargo")
val rustcWrapper = rustWrapperDir.resolve(if (isWindows) "rustc.bat" else "rustc")

fun writeRustWrapper(
    scriptFile: File,
    executable: File,
    exportedRustc: File? = null,
    extraPathDirs: List<File> = emptyList(),
) {
    val pathValue = withPrependedPath(*extraPathDirs.toTypedArray())

    val lines = if (isWindows) {
        buildList {
            add("@echo off")
            add("set \"PATH=${pathValue}\"")
            if (cargoHome != null) {
                add("set \"CARGO_HOME=${cargoHome.absolutePath}\"")
            }
            if (rustupHome != null) {
                add("set \"RUSTUP_HOME=${rustupHome.absolutePath}\"")
            }
            add("set \"RUSTUP_TOOLCHAIN=${cargoToolchain}\"")
            if (exportedRustc != null) {
                add("set \"RUSTC=${exportedRustc.absolutePath}\"")
            }
            add("\"${executable.absolutePath}\" %*")
        }
    } else {
        buildList {
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
    }

    scriptFile.parentFile.mkdirs()
    val separator = if (isWindows) "\r\n" else "\n"
    scriptFile.writeText(lines.joinToString(separator, postfix = separator))
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

fun Any.setCargoProperty(name: String, value: Any?) {
    val setter = javaClass.methods.firstOrNull {
        it.name == "set${name.replaceFirstChar(Char::titlecaseChar)}" && it.parameterCount == 1
    } ?: throw GradleException("Cargo extension setter not found: $name")
    setter.invoke(this, value)
}

val cargoExtension = extensions.getByName("cargo")
cargoExtension.setCargoProperty("module", coreffiDir.absolutePath)
cargoExtension.setCargoProperty("libname", "uniffi_synap_coreffi")
cargoExtension.setCargoProperty("targets", cargoPluginTargets)
cargoExtension.setCargoProperty("profile", cargoProfile)
cargoExtension.setCargoProperty("targetDirectory", repoRootDir.resolve("target").absolutePath)
cargoExtension.setCargoProperty("apiLevel", 24)
cargoExtension.setCargoProperty("prebuiltToolchains", true)

if (cargoWrapper.exists()) {
    cargoExtension.setCargoProperty("cargoCommand", cargoWrapper.absolutePath)
} else if (cargoExecutable != null) {
    cargoExtension.setCargoProperty("cargoCommand", cargoExecutable.absolutePath)
}

if (rustcWrapper.exists()) {
    cargoExtension.setCargoProperty("rustcCommand", rustcWrapper.absolutePath)
} else if (rustcExecutable != null) {
    cargoExtension.setCargoProperty("rustcCommand", rustcExecutable.absolutePath)
}

if (toolchainCargoExecutable == null) {
    cargoExtension.setCargoProperty("rustupChannel", cargoChannel)
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
