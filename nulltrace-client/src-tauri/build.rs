fn main() {
    tauri_build::build();
    tonic_build::compile_protos("../proto/game.proto").expect("Failed to compile game.proto");
}
