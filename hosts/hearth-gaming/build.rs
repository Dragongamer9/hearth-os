fn main() {
    // The Hearth Mod Organizer's end-to-end overlay test runs only where fuse-overlayfs is installed (and is listed
    // as ignored elsewhere, so the test summary shows whether it ran).
    println!("cargo::rustc-check-cfg=cfg(has_fuse_overlayfs)");
    println!("cargo:rerun-if-env-changed=PATH");
    let found = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join("fuse-overlayfs").is_file())).unwrap_or(false);
    if found && std::path::Path::new("/dev/fuse").exists() { println!("cargo:rustc-cfg=has_fuse_overlayfs"); }
    tauri_build::build()
}
