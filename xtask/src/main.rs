use std::env;

use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use uniffi_bindgen::{generate_bindings, bindings::KotlinBindingGenerator};

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
        Some(cmd) => bail!("unknown xtask command: {cmd}"),
        None => bail!("missing xtask command"),
    }
}

fn generate_uniffi_kotlin(args: Vec<String>) -> Result<()> {
    let mut udl: Option<Utf8PathBuf> = None;
    let mut config: Option<Utf8PathBuf> = None;
    let mut out_dir: Option<Utf8PathBuf> = None;

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
            flag => bail!("unknown flag for gen-uniffi-kotlin: {flag}"),
        }
    }

    let udl = udl.context("missing --udl")?;
    let out_dir = out_dir.context("missing --out-dir")?;

    generate_bindings(
        &udl,
        config.as_deref(),
        KotlinBindingGenerator,
        Some(&out_dir),
        None,
        None,
        false,
    )
    .context("failed to generate Kotlin UniFFI bindings")?;

    Ok(())
}
