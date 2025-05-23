fn main() {
    capnpc::CompilerCommand::new()
        .file("../proto/schema.capnp")
        .output_path("src/")
        .run()
        .expect("Error compiling capnp schema");
}
