fn main() {
    prost_build::compile_protos(&["src/request.proto"], &["src/"]).unwrap();
}
