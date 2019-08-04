#[cfg(feature = "grpc")]
extern crate tower_grpc_build;

#[cfg(feature = "grpc")]
fn main() {
    /*tower_grpc_build::Config::new()
    .enable_server(false)
    .enable_client(true)
    .build(&["proto/miner.proto"], &["proto"])
    .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));*/

    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(false)
        .build(&["proto/node.proto"], &["proto"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}

#[cfg(not(feature = "grpc"))]
fn main() {}
