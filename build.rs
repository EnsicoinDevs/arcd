fn main() {
    #[cfg(feature = "grpc")]
    tonic_build::compile_protos("proto/node.proto").expect("protobuf creation failed")
}
