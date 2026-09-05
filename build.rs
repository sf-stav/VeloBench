use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/velobench.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/velobench.proto");
    Ok(())
}
