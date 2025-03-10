fn main() {
    protobuf_codegen::Codegen::new()
        .cargo_out_dir("test-protos")
        .include("test-protos")
        .input("test-protos/types.proto")
        .run_from_script();
}

// fn main() {
//     protobuf_codegen::Codegen::new()
//         .cargo_out_dir("protos")
//         .include("test-protos")
//         .input("test-protos/types.proto")
//         .run_from_script();
// }
