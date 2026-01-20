//! Build script for generating gRPC code from MinKNOW protobufs.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to rerun this build script if proto files change
    println!("cargo:rerun-if-changed=proto/");

    // Compile the proto files we need for the MVP
    // Start with manager.proto which imports device, instance, protocol_settings, rpc_options
    tonic_build::configure()
        .build_server(false) // We're only a client
        .build_client(true)
        .compile_protos(
            &[
                "proto/minknow_api/manager.proto",
                "proto/minknow_api/acquisition.proto",
                "proto/minknow_api/statistics.proto",
                "proto/minknow_api/instance.proto",
            ],
            &["proto/"],
        )?;

    Ok(())
}
