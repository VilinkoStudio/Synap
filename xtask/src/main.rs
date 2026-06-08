use std::{env, fs, process::Command};

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;
use uniffi_bindgen_029 as csharp_uniffi_bindgen;
use uniffi_bindgen_031::bindings::{
    generate as generate_language_bindings, GenerateOptions, TargetLanguage,
};
use uniffi_bindgen_cs::{gen_cs, generate_bindings as generate_csharp_bindings};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("gen-uniffi-kotlin") => generate_uniffi_kotlin(args.collect()),
        Some("gen-uniffi-csharp") => generate_uniffi_csharp(args.collect()),
        Some("gen-uniffi-node") => generate_uniffi_node(args.collect()),
        Some(cmd) => bail!("unknown xtask command: {cmd}"),
        None => bail!("missing xtask command"),
    }
}

struct GenerateBindingsArgs {
    udl: Utf8PathBuf,
    config: Option<Utf8PathBuf>,
    out_dir: Utf8PathBuf,
    crate_name: Option<String>,
}

struct GenerateNodeArgs {
    udl: Utf8PathBuf,
    config: Utf8PathBuf,
    out_dir: Utf8PathBuf,
    crate_name: String,
    package_name: String,
    rust_package: String,
    node_modules_dir: Option<Utf8PathBuf>,
    manual_load: bool,
}

fn parse_generate_bindings_args(
    args: Vec<String>,
    command_name: &str,
) -> Result<GenerateBindingsArgs> {
    let mut udl: Option<Utf8PathBuf> = None;
    let mut config: Option<Utf8PathBuf> = None;
    let mut out_dir: Option<Utf8PathBuf> = None;
    let mut crate_name: Option<String> = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--udl" => {
                let value = iter.next().context("missing value for --udl")?;
                udl = Some(Utf8PathBuf::from(value));
            }
            "--config" => {
                let value = iter.next().context("missing value for --config")?;
                config = Some(Utf8PathBuf::from(value));
            }
            "--out-dir" => {
                let value = iter.next().context("missing value for --out-dir")?;
                out_dir = Some(Utf8PathBuf::from(value));
            }
            "--crate-name" => {
                let value = iter.next().context("missing value for --crate-name")?;
                validate_crate_name(&value)?;
                crate_name = Some(value);
            }
            flag => bail!("unknown flag for {command_name}: {flag}"),
        }
    }

    Ok(GenerateBindingsArgs {
        udl: udl.context("missing --udl")?,
        config,
        out_dir: out_dir.context("missing --out-dir")?,
        crate_name,
    })
}

fn parse_generate_node_args(args: Vec<String>) -> Result<GenerateNodeArgs> {
    let mut udl: Option<Utf8PathBuf> = None;
    let mut config: Option<Utf8PathBuf> = None;
    let mut out_dir: Option<Utf8PathBuf> = None;
    let mut crate_name: Option<String> = None;
    let mut package_name: Option<String> = None;
    let mut rust_package: Option<String> = None;
    let mut node_modules_dir: Option<Utf8PathBuf> = None;
    let mut manual_load = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--udl" => {
                let value = iter.next().context("missing value for --udl")?;
                udl = Some(Utf8PathBuf::from(value));
            }
            "--config" => {
                let value = iter.next().context("missing value for --config")?;
                config = Some(Utf8PathBuf::from(value));
            }
            "--out-dir" => {
                let value = iter.next().context("missing value for --out-dir")?;
                out_dir = Some(Utf8PathBuf::from(value));
            }
            "--crate-name" => {
                let value = iter.next().context("missing value for --crate-name")?;
                validate_crate_name(&value)?;
                crate_name = Some(value);
            }
            "--package-name" => {
                package_name = Some(iter.next().context("missing value for --package-name")?);
            }
            "--rust-package" => {
                rust_package = Some(iter.next().context("missing value for --rust-package")?);
            }
            "--node-modules-dir" => {
                let value = iter
                    .next()
                    .context("missing value for --node-modules-dir")?;
                node_modules_dir = Some(Utf8PathBuf::from(value));
            }
            "--manual-load" => {
                manual_load = true;
            }
            flag => bail!("unknown flag for gen-uniffi-node: {flag}"),
        }
    }

    Ok(GenerateNodeArgs {
        udl: udl.context("missing --udl")?,
        config: config.context("missing --config")?,
        out_dir: out_dir.context("missing --out-dir")?,
        crate_name: crate_name.context("missing --crate-name")?,
        package_name: package_name.context("missing --package-name")?,
        rust_package: rust_package.context("missing --rust-package")?,
        node_modules_dir,
        manual_load,
    })
}

fn generate_uniffi_kotlin(args: Vec<String>) -> Result<()> {
    let parsed = parse_generate_bindings_args(args, "gen-uniffi-kotlin")?;
    let source = match parsed.crate_name.as_deref() {
        Some(crate_name) => {
            prepare_adapter_udl_input(AdapterUdlInput {
                udl: &parsed.udl,
                config: None,
                crate_name,
                package_name: "synap-uniffi-input",
                input_key: crate_name,
                crate_type: None,
                write_lib_rs: false,
            })?
            .udl_path
        }
        None => parsed.udl,
    };

    generate_language_bindings(GenerateOptions {
        languages: vec![TargetLanguage::Kotlin],
        source,
        out_dir: parsed.out_dir,
        config_override: parsed.config,
        format: false,
        crate_filter: None,
        metadata_no_deps: false,
    })
    .context("failed to generate Kotlin UniFFI bindings")?;

    Ok(())
}

fn generate_uniffi_csharp(args: Vec<String>) -> Result<()> {
    let parsed = parse_generate_bindings_args(args, "gen-uniffi-csharp")?;

    csharp_uniffi_bindgen::generate_external_bindings(
        &CSharpBindingGenerator {
            try_format_code: false,
        },
        &parsed.udl,
        parsed.config.as_deref(),
        Some(&parsed.out_dir),
        None::<&Utf8Path>,
        parsed.crate_name.as_deref(),
        false,
    )
    .context("failed to generate C# UniFFI bindings")?;

    Ok(())
}

fn generate_uniffi_node(args: Vec<String>) -> Result<()> {
    let parsed = parse_generate_node_args(args)?;
    validate_crate_name(&parsed.crate_name)?;

    let lib_file = build_and_resolve_cdylib(&parsed.rust_package)?;
    let input = prepare_adapter_udl_input(AdapterUdlInput {
        udl: &parsed.udl,
        config: Some(&parsed.config),
        crate_name: &parsed.crate_name,
        package_name: "synap-coreffi-node-bindgen-input",
        input_key: "nodejs-synap-coreffi",
        crate_type: Some("cdylib"),
        write_lib_rs: true,
    })?;

    if parsed.out_dir.exists() {
        fs::remove_dir_all(&parsed.out_dir)
            .with_context(|| format!("failed to remove stale {}", parsed.out_dir))?;
    }

    let mut command = Command::new("uniffi-bindgen-node-js");
    command
        .arg("generate")
        .arg(&lib_file)
        .arg("--out-dir")
        .arg(&parsed.out_dir)
        .arg("--crate-name")
        .arg(&parsed.crate_name)
        .arg("--package-name")
        .arg(&parsed.package_name)
        .arg("--manifest-path")
        .arg(&input.manifest_path);
    if parsed.manual_load {
        command.arg("--manual-load");
    }
    run_command(command).context("failed to generate Node UniFFI bindings")?;

    fs::create_dir_all(&parsed.out_dir)
        .with_context(|| format!("failed to create {}", parsed.out_dir))?;
    fs::copy(
        &lib_file,
        parsed
            .out_dir
            .join(lib_file.file_name().unwrap_or("cdylib")),
    )
    .with_context(|| format!("failed to copy {} into {}", lib_file, parsed.out_dir))?;

    if let Some(node_modules_dir) = parsed.node_modules_dir {
        link_node_modules(&parsed.out_dir, &node_modules_dir)?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct CargoMessage {
    reason: Option<String>,
    package_id: Option<String>,
    target: Option<CargoTarget>,
    filenames: Option<Vec<Utf8PathBuf>>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    kind: Vec<String>,
}

fn build_and_resolve_cdylib(rust_package: &str) -> Result<Utf8PathBuf> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg(rust_package)
        .arg("--message-format=json")
        .output()
        .context("failed to run cargo build for Node UniFFI cdylib")?;

    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        bail!(
            "cargo build -p {rust_package} --message-format=json failed with code {:?}",
            output.status.code()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("cargo JSON output is not UTF-8")?;
    find_cdylib_artifact_path(&stdout, rust_package)
        .with_context(|| format!("failed to find cdylib artifact for package {rust_package}"))
}

fn find_cdylib_artifact_path(output: &str, rust_package: &str) -> Result<Utf8PathBuf> {
    for line in output.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }

        let message: CargoMessage = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(_) => continue,
        };

        if message.reason.as_deref() != Some("compiler-artifact")
            || !message.package_id.as_deref().is_some_and(|id| {
                id.contains(&format!("#{rust_package}@"))
                    || id.contains(&format!("/{rust_package}#"))
            })
            || !message
                .target
                .as_ref()
                .is_some_and(|target| target.kind.iter().any(|kind| kind == "cdylib"))
        {
            continue;
        }

        if let Some(path) = message
            .filenames
            .unwrap_or_default()
            .into_iter()
            .find(|path| is_native_library_path(path))
        {
            return Ok(path);
        }
    }

    bail!("no cdylib compiler artifact found in cargo output")
}

fn is_native_library_path(path: &Utf8Path) -> bool {
    if cfg!(target_os = "macos") {
        path.extension() == Some("dylib")
    } else if cfg!(target_os = "windows") {
        path.extension() == Some("dll")
    } else {
        path.extension() == Some("so")
    }
}

fn run_command(mut command: Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to spawn {:?}", command))?;
    if !status.success() {
        bail!("command {:?} failed with code {:?}", command, status.code());
    }
    Ok(())
}

fn link_node_modules(out_dir: &Utf8Path, node_modules_dir: &Utf8Path) -> Result<()> {
    if !node_modules_dir.is_dir() {
        bail!(
            "node_modules directory not found at {}. Install web dependencies first.",
            node_modules_dir
        );
    }

    let generated_node_modules = out_dir.join("node_modules");
    if generated_node_modules.exists() {
        let metadata = fs::symlink_metadata(&generated_node_modules)
            .with_context(|| format!("failed to inspect {}", generated_node_modules))?;
        if metadata.file_type().is_symlink() {
            fs::remove_file(&generated_node_modules)
                .with_context(|| format!("failed to remove {}", generated_node_modules))?;
        } else {
            return Ok(());
        }
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(node_modules_dir, &generated_node_modules).with_context(
            || {
                format!(
                    "failed to link {} to {}",
                    generated_node_modules, node_modules_dir
                )
            },
        )?;
    }

    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(node_modules_dir, &generated_node_modules).with_context(
            || {
                format!(
                    "failed to link {} to {}",
                    generated_node_modules, node_modules_dir
                )
            },
        )?;
    }

    Ok(())
}

fn validate_crate_name(crate_name: &str) -> Result<()> {
    if crate_name.is_empty()
        || !crate_name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        bail!("invalid --crate-name: {crate_name}");
    }
    Ok(())
}

struct AdapterUdlInput<'a> {
    udl: &'a Utf8Path,
    config: Option<&'a Utf8Path>,
    crate_name: &'a str,
    package_name: &'a str,
    input_key: &'a str,
    crate_type: Option<&'a str>,
    write_lib_rs: bool,
}

struct PreparedAdapterUdlInput {
    udl_path: Utf8PathBuf,
    manifest_path: Utf8PathBuf,
}

fn prepare_adapter_udl_input(input: AdapterUdlInput<'_>) -> Result<PreparedAdapterUdlInput> {
    let repo_root = Utf8PathBuf::from_path_buf(env::current_dir()?)
        .map_err(|path| anyhow::anyhow!("current directory is not UTF-8: {}", path.display()))?;
    let input_root = repo_root
        .join("target")
        .join("xtask")
        .join("uniffi-input")
        .join(input.input_key);
    let input_src = input_root.join("src");
    let input_udl = input_src.join(
        input
            .udl
            .file_name()
            .context("UDL path does not have a file name")?,
    );
    let manifest_path = input_root.join("Cargo.toml");

    if input_root.exists() {
        fs::remove_dir_all(&input_root)
            .with_context(|| format!("failed to remove stale {}", input_root))?;
    }
    fs::create_dir_all(&input_src).with_context(|| format!("failed to create {}", input_src))?;
    fs::copy(input.udl, &input_udl).with_context(|| {
        format!(
            "failed to copy shared UDL from {} to {}",
            input.udl, input_udl
        )
    })?;
    if let Some(config) = input.config {
        fs::copy(config, input_root.join("uniffi.toml"))
            .with_context(|| format!("failed to copy UniFFI config from {}", config))?;
    }
    if input.write_lib_rs {
        fs::write(
            input_src.join("lib.rs"),
            "// Generated input crate for UniFFI binding metadata resolution.\n",
        )
        .with_context(|| format!("failed to write {}", input_src.join("lib.rs")))?;
    }

    let crate_type = input
        .crate_type
        .map(|kind| format!("crate-type = [\"{kind}\"]\n"))
        .unwrap_or_default();
    fs::write(
        &manifest_path,
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\nname = \"{}\"\n{}\
             \n[workspace]\n",
            input.package_name, input.crate_name, crate_type
        ),
    )
    .with_context(|| format!("failed to write {}", manifest_path))?;

    Ok(PreparedAdapterUdlInput {
        udl_path: input_udl,
        manifest_path,
    })
}

struct CSharpBindingGenerator {
    try_format_code: bool,
}

impl csharp_uniffi_bindgen::BindingGenerator for CSharpBindingGenerator {
    type Config = gen_cs::Config;

    fn new_config(&self, root_toml: &toml::Value) -> Result<Self::Config> {
        Ok(
            match root_toml.get("bindings").and_then(|b| b.get("csharp")) {
                Some(v) => gen_cs::Config::deserialize(v.clone())?,
                None => Default::default(),
            },
        )
    }

    fn write_bindings(
        &self,
        settings: &csharp_uniffi_bindgen::GenerationSettings,
        components: &[csharp_uniffi_bindgen::Component<Self::Config>],
    ) -> Result<()> {
        for csharp_uniffi_bindgen::Component { ci, config, .. } in components {
            let bindings_file = settings.out_dir.join(format!("{}.cs", ci.namespace()));
            let mut bindings = generate_csharp_bindings(config, ci)?;

            bindings = gen_cs::formatting::add_header(bindings);
            std::fs::write(&bindings_file, bindings)
                .with_context(|| format!("failed to write {}", bindings_file))?;

            if self.try_format_code {
                gen_cs::formatting::format(&bindings_file)
                    .with_context(|| format!("failed to format {}", bindings_file))?;
            }
        }

        Ok(())
    }

    fn update_component_configs(
        &self,
        _settings: &csharp_uniffi_bindgen::GenerationSettings,
        _components: &mut Vec<csharp_uniffi_bindgen::Component<Self::Config>>,
    ) -> Result<()> {
        Ok(())
    }
}
