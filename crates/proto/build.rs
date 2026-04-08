fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir("src")
        .compile_protos(&["proto/scheduler.proto", "proto/api.proto", "proto/notificator.proto", "proto/rotator.proto"], &["proto"])?;

    Ok(())
}
