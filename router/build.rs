fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("../proto/")
        .file("../proto/schema.capnp")
        .output_path("src/rpc")
        .run()
        .expect("Error compiling capnp schema");
}
