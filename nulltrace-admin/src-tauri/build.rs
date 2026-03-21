fn main() {
    tauri_build::build();
    tonic_build::compile_protos("../proto/admin.proto").expect("Failed to compile admin.proto");
}
