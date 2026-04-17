use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Compile FlatBuffers schemas to Rust
    flatbuffers_build::compile_flatbuffers_files(
        &["proto/schema.fbs", "proto/metadata.fbs"],
        &["proto/"],
        &out_dir,
    )
    .expect("Failed to compile FlatBuffers schemas");

    println!("cargo:rerun-if-changed=proto/");
}
