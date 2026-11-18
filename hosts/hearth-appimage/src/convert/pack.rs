//! Packing an AppDir into an AppImage: AppImage's type 2 runtime (the small program at the front that mounts the
//! rest) followed by a zstd squashfs of the AppDir, written here in Rust (backhand).

use backhand::compression::Compressor;
use backhand::{FilesystemCompressor, FilesystemWriter, NodeHeader};
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn arch_name(machine: u16) -> &'static str { if machine == 183 { "aarch64" } else { "x86_64" } }

const RUNTIME_URL: &str = "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-";

fn good_runtime(p: &Path) -> bool {
    let mut m = [0u8; 4];
    fs::metadata(p).map(|m| m.len() > 100_000).unwrap_or(false)
        && fs::File::open(p).and_then(|mut f| f.read_exact(&mut m)).is_ok() && m == *b"\x7fELF"
}

/// The runtime for this machine type: $HEARTH_APPIMAGE_RUNTIME, the copy Hearth OS ships, the one downloaded
/// before, or downloaded now (once) from AppImage's own releases.
pub fn runtime(machine: u16, log: &mut dyn FnMut(String)) -> io::Result<PathBuf> {
    let arch = arch_name(machine);
    let mut places: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("HEARTH_APPIMAGE_RUNTIME") { places.push(PathBuf::from(p)); }
    places.push(PathBuf::from(format!("/usr/share/hearth-appimage/runtime-{arch}")));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() { places.push(d.join(format!("runtime-{arch}"))); }
    }
    let cache = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("hearth-appimage").join(format!("runtime-{arch}"));
    places.push(cache.clone());
    if let Some(p) = places.iter().find(|p| good_runtime(p)) { return Ok(p.clone()); }
    log(format!("Downloading the AppImage runtime ({arch}) once"));
    fs::create_dir_all(cache.parent().unwrap())?;
    let resp = ureq::get(&format!("{RUNTIME_URL}{arch}")).call().map_err(|e| io::Error::other(format!("couldn't download the AppImage runtime: {e}")))?;
    let tmp = cache.with_extension("part");
    let mut f = fs::File::create(&tmp)?;
    io::copy(&mut resp.into_reader().take(64 << 20), &mut f)?;
    f.sync_all()?;
    drop(f);
    if !good_runtime(&tmp) { let _ = fs::remove_file(&tmp); return Err(io::Error::other("the downloaded AppImage runtime isn't a program")); }
    fs::rename(&tmp, &cache)?;
    Ok(cache)
}

/// The squashfs of `appdir`, written to `out`. Everything owned by root, times kept, modes as they are.
pub fn squash(appdir: &Path, out: &Path) -> io::Result<()> {
    let e = |x: backhand::BackhandError| io::Error::other(x.to_string());
    let mut fsw = FilesystemWriter::default();
    fsw.set_compressor(FilesystemCompressor::new(Compressor::Zstd, None).map_err(e)?);
    fsw.set_root_mode(0o755);
    fsw.set_current_time();
    for entry in WalkDir::new(appdir).follow_links(false).sort_by_file_name().min_depth(1) {
        let entry = entry.map_err(|x| io::Error::other(x.to_string()))?;
        let rel = entry.path().strip_prefix(appdir).unwrap();
        let path = Path::new("/").join(rel);
        let md = entry.path().symlink_metadata()?;
        let mode = (md.permissions().mode() & 0o7777) as u16;
        let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as u32).unwrap_or(0);
        let h = NodeHeader::new(mode, 0, 0, mtime);
        let ft = entry.file_type();
        if ft.is_dir() {
            fsw.push_dir(&path, h).map_err(e)?;
        } else if ft.is_symlink() {
            fsw.push_symlink(fs::read_link(entry.path())?, &path, h).map_err(e)?;
        } else if ft.is_file() {
            fsw.push_file_from_path(entry.path().to_path_buf(), &path, h).map_err(e)?;
        }
    }
    let mut f = fs::File::create(out)?;
    fsw.write(&mut f).map_err(e)?;
    f.sync_all()
}

/// runtime + squashfs → the AppImage at `out` (written beside it first, then moved into place).
pub fn appimage(appdir: &Path, runtime: &Path, out: &Path) -> io::Result<()> {
    let dir = out.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let sq = out.with_extension("squashfs.part");
    squash(appdir, &sq)?;
    let part = out.with_extension("AppImage.part");
    {
        let mut o = fs::File::create(&part)?;
        io::copy(&mut fs::File::open(runtime)?, &mut o)?;
        io::copy(&mut fs::File::open(&sq)?, &mut o)?;
        o.flush()?;
        o.sync_all()?;
    }
    let _ = fs::remove_file(&sq);
    fs::set_permissions(&part, fs::Permissions::from_mode(0o755))?;
    fs::rename(&part, out)
}
