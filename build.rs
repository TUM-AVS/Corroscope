extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["src/commonroad_pb/commonroad.proto"],
                                &["src/commonroad_pb"]).unwrap();
}