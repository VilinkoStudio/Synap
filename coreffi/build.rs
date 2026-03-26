fn main() {
    // Tell cargo to invalidate the built crate whenever the UDL changes
    println!("cargo:rerun-if-changed=src/synap.udl");

    // Generate uniffi scaffolding
    uniffi_build::generate_scaffolding("src/synap.udl").expect("Building uniffi scaffolding failed");
}
