fn main() {
    println!("cargo:rerun-if-changed=../coreffi-shared/src/synap.udl");

    uniffi_build::generate_scaffolding_for_crate(
        "../coreffi-shared/src/synap.udl",
        "uniffi_synap_coreffi_uniffi029",
    )
    .expect("Building uniffi scaffolding failed");
}
