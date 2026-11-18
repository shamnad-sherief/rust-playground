fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }

    tonic_prost_build::compile_protos("proto/calculator.proto")?;

    println!("cargo:rerun-if-changed=proto/calculator.proto");

    Ok(())
}
