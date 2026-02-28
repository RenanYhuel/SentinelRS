fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let proto_path = format!("{}/proto/sentinel.proto", manifest_dir);
    let proto_dir = format!("{}/proto", manifest_dir);

    prost_build::compile_protos(&[proto_path], &[proto_dir])
        .expect("Failed to compile proto files with prost-build");

    println!("cargo:rerun-if-changed={}/proto/sentinel.proto", std::env::var("CARGO_MANIFEST_DIR").unwrap());
}
