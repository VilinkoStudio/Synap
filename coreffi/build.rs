fn main() {
    // Tell cargo to invalidate the built crate whenever the UDL changes
    println!("cargo:rerun-if-changed=../coreffi-shared/src/synap.udl");

    // Generate UniFFI scaffolding with the adapter crate name. The UDL lives in
    // coreffi-shared, but this cdylib exports the `uniffi_synap_coreffi` symbols.
    uniffi_build::generate_scaffolding_for_crate(
        "../coreffi-shared/src/synap.udl",
        "uniffi_synap_coreffi",
    )
    .expect("Building uniffi scaffolding failed");
}
