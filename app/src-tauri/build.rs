fn main() {
    // `tauri::generate_context!` requires the frontend output to exist. Create
    // an empty one so `cargo build --workspace` works before `pnpm build`.
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(&dist);
        let _ = std::fs::write(dist.join("index.html"), "<!doctype html><p>Run `pnpm build` in app/.</p>");
    }
    tauri_build::build()
}
