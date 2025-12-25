fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/depot.proto")?;
    tonic_build::compile_protos("proto/indicators.proto")?;
    Ok(())
}
