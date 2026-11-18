fn main() {
    // The window's build (the default); the command-line-only build (--no-default-features) needs nothing.
    #[cfg(feature = "gui")]
    tauri_build::build();
}
