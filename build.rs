extern crate prost_build;

fn main() {
    const PROTO_DIR: &'static str = "src/commonroad_pb";
    const PROTO_FILE: &'static str = "src/commonroad_pb/commonroad.proto";

    println!("cargo:rerun-if-changed={}", PROTO_DIR);

    prost_build::compile_protos(
        &[PROTO_FILE],
        &["src/commonroad_pb"],
    )
    .unwrap();
}
