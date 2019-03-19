fn main() {
    prost_build::compile_protos(
        &[
            "src/protobuf/types.proto",
            "src/protobuf/node.proto",
            "src/protobuf/miner.proto",
        ],
        &["src/protobuf"],
    )
    .unwrap();
}
