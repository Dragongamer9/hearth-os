//! The libraries an app needs, found by reading its programs (ELF DT_NEEDED and GLIBC_ version needs) without running
//! them, then put in the AppImage: from this computer when it has them, else downloaded from Fedora's repositories
//! with `dnf download` (no root, nothing installed). The base system every Linux has (glibc, the GPU drivers'
//! libraries, X11/Wayland, ALSA/PipeWire, fontconfig) is left out, as AppImages always do: those must come from the
//! computer the AppImage runs on, or graphics and sound break.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Never bundled (AppImage's excludelist, the parts that matter).
const BASE: &[&str] = &[
    "ld-linux-x86-64.so.2", "ld-linux-aarch64.so.1", "ld-linux.so.2", "libc.so.6", "libm.so.6", "libdl.so.2",
    "libpthread.so.0", "librt.so.1", "libresolv.so.2", "libutil.so.1", "libanl.so.1", "libmvec.so.1",
    "libBrokenLocale.so.1", "libthread_db.so.1", "libnsl.so.1", "libnss_files.so.2", "libnss_dns.so.2", "libc_malloc_debug.so.0",
    "libgcc_s.so.1",
    "libGL.so.1", "libEGL.so.1", "libGLX.so.0", "libGLdispatch.so.0", "libOpenGL.so.0", "libGLESv2.so.2", "libGLESv1_CM.so.1",
    "libglapi.so.0", "libgbm.so.1", "libdrm.so.2", "libvulkan.so.1", "libcuda.so.1", "libnvidia-ml.so.1", "libOpenCL.so.1",
    "libva.so.2", "libva-drm.so.2", "libva-x11.so.2", "libva-wayland.so.2", "libvdpau.so.1",
    "libX11.so.6", "libX11-xcb.so.1", "libxcb.so.1", "libxcb-dri2.so.0", "libxcb-dri3.so.0", "libxcb-glx.so.0", "libxcb-present.so.0",
    "libxcb-sync.so.1", "libxshmfence.so.1", "libXau.so.6", "libXdmcp.so.6", "libICE.so.6", "libSM.so.6",
    "libwayland-client.so.0", "libwayland-server.so.0", "libwayland-egl.so.1", "libwayland-cursor.so.0",
    "libasound.so.2", "libjack.so.0", "libpipewire-0.3.so.0", "libpulse.so.0", "libpulse-simple.so.0",
    "libfontconfig.so.1", "libfreetype.so.6", "libharfbuzz.so.0", "libexpat.so.1", "libz.so.1",
    "libdbus-1.so.3", "libudev.so.1", "libsystemd.so.0", "libusb-1.0.so.0", "libuuid.so.1", "libcom_err.so.2",
    "libgpg-error.so.0", "libselinux.so.1",
];

pub fn is_base(soname: &str) -> bool {
    BASE.contains(&soname) || soname.starts_with("libnvidia-") || soname.starts_with("libnss_")
}

/// One program or library in the app.
#[derive(Debug, Default, Clone)]
pub struct Elf {
    pub needed: Vec<String>,
    pub glibc: Option<(u32, u32)>,
    pub machine: u16,
    pub is_lib: bool,
    pub soname: Option<String>,
}

pub fn is_elf(path: &Path) -> bool {
    let mut m = [0u8; 4];
    fs::File::open(path).and_then(|mut f| f.read_exact(&mut m)).is_ok() && m == *b"\x7fELF"
}

fn glibc_version(name: &str) -> Option<(u32, u32)> {
    let v = name.strip_prefix("GLIBC_")?;
    let mut p = v.split('.');
    Some((p.next()?.parse().ok()?, p.next().and_then(|x| x.parse().ok()).unwrap_or(0)))
}

/// Reads one ELF file. Files that aren't ELF, or that goblin can't read, give None.
pub fn read_elf(path: &Path) -> Option<Elf> {
    if !is_elf(path) { return None; }
    let bytes = fs::read(path).ok()?;
    let e = goblin::elf::Elf::parse(&bytes).ok()?;
    let mut glibc = None;
    if let Some(vn) = &e.verneed {
        for need in vn.iter() {
            for aux in need.iter() {
                if let Some(v) = e.dynstrtab.get_at(aux.vna_name).and_then(glibc_version) {
                    glibc = glibc.max(Some(v));
                }
            }
        }
    }
    Some(Elf {
        needed: e.libraries.iter().map(|s| s.to_string()).collect(),
        glibc,
        machine: e.header.e_machine,
        is_lib: e.header.e_type == goblin::elf::header::ET_DYN && e.interpreter.is_none(),
        soname: e.soname.map(|s| s.to_string()),
    })
}

/// The libraries this computer's loader knows (`ldconfig -p`), for the machine type wanted.
pub fn host_libs(machine: u16) -> HashMap<String, PathBuf> {
    let want = match machine { 183 => "AArch64", _ => "x86-64" };
    let mut map = HashMap::new();
    for ldconfig in ["/usr/sbin/ldconfig", "/sbin/ldconfig", "ldconfig"] {
        let Ok(o) = Command::new(ldconfig).arg("-p").output() else { continue };
        if !o.status.success() { continue; }
        for line in String::from_utf8_lossy(&o.stdout).lines().skip(1) {
            // \tlibz.so.1 (libc6,x86-64) => /lib64/libz.so.1
            let Some((left, path)) = line.split_once("=>") else { continue };
            let left = left.trim();
            let Some((name, flags)) = left.split_once(' ') else { continue };
            let ok = if want == "x86-64" { flags.contains("x86-64") } else { flags.contains(want) };
            if ok { map.entry(name.to_string()).or_insert_with(|| PathBuf::from(path.trim())); }
        }
        break;
    }
    map
}

pub fn host_glibc() -> Option<(u32, u32)> {
    let o = Command::new("getconf").arg("GNU_LIBC_VERSION").output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout);
    let v = s.split_whitespace().nth(1)?;
    glibc_version(&format!("GLIBC_{v}"))
}

/// Every ELF file in the folder (links not followed).
pub fn scan(root: &Path) -> BTreeMap<PathBuf, Elf> {
    let mut out = BTreeMap::new();
    for e in WalkDir::new(root).follow_links(false).into_iter().flatten() {
        if !e.file_type().is_file() { continue; }
        if let Some(elf) = read_elf(e.path()) { out.insert(e.path().to_path_buf(), elf); }
    }
    out
}

/// What resolving found.
#[derive(Debug, Default)]
pub struct Found {
    /// Copied from this computer.
    pub from_host: Vec<String>,
    /// Downloaded from Fedora: (library, package).
    pub from_fedora: Vec<(String, String)>,
    /// Nowhere to be had.
    pub missing: Vec<String>,
    /// The newest glibc any program in it needs.
    pub glibc: Option<(u32, u32)>,
    pub machine: u16,
    /// Folders inside the app that hold libraries (relative), for AppRun's LD_LIBRARY_PATH.
    pub lib_dirs: Vec<PathBuf>,
}

/// Puts everything the app's programs need (and what those need) into `appdir/usr/lib`, except the base system.
/// `fetch`: ask Fedora's repositories for what this computer doesn't have. `log` gets a line per step.
pub fn bundle(appdir: &Path, fetch: bool, log: &mut dyn FnMut(String)) -> io::Result<Found> {
    let mut found = Found::default();
    let libdir = appdir.join("usr/lib");
    fs::create_dir_all(&libdir)?;
    let mut elves = scan(appdir);
    found.machine = elves.values().map(|e| e.machine).find(|m| *m != 0).unwrap_or(62);
    let host = host_libs(found.machine);
    let mut tried_fedora: BTreeSet<String> = BTreeSet::new();
    // Rounds: what was added can need more.
    for _round in 0..12 {
        let mut have: BTreeSet<String> = BTreeSet::new();
        for (p, e) in &elves {
            if let Some(n) = p.file_name() { have.insert(n.to_string_lossy().into_owned()); }
            if let Some(s) = &e.soname { have.insert(s.clone()); }
        }
        // Links in the app named like libraries (libfoo.so.1 -> libfoo.so.1.2.3).
        for e in WalkDir::new(appdir).follow_links(false).into_iter().flatten() {
            if e.file_type().is_symlink() { have.insert(e.file_name().to_string_lossy().into_owned()); }
        }
        let mut wanted: BTreeSet<String> = BTreeSet::new();
        for e in elves.values() {
            for n in &e.needed { if !have.contains(n) && !is_base(n) { wanted.insert(n.clone()); } }
        }
        wanted.retain(|n| !found.missing.contains(n));
        if wanted.is_empty() { break; }
        let mut added = false;
        let mut still: Vec<String> = Vec::new();
        for n in &wanted {
            if let Some(src) = host.get(n) {
                let dst = libdir.join(n);
                if fs::copy(src, &dst).is_ok() {
                    log(format!("Library from this computer: {n}"));
                    found.from_host.push(n.clone());
                    if let Some(e) = read_elf(&dst) { elves.insert(dst, e); }
                    added = true;
                    continue;
                }
            }
            still.push(n.clone());
        }
        if fetch && !still.is_empty() {
            let ask: Vec<String> = still.iter().filter(|n| !tried_fedora.contains(*n)).cloned().collect();
            for n in &ask { tried_fedora.insert(n.clone()); }
            if !ask.is_empty() {
                for (lib, pkg, dst) in fedora_fetch(&ask, found.machine, &libdir, log) {
                    found.from_fedora.push((lib.clone(), pkg));
                    still.retain(|x| x != &lib);
                    if let Some(e) = read_elf(&dst) { elves.insert(dst, e); }
                    added = true;
                }
            }
        }
        if !added {
            for n in still { log(format!("Not found anywhere: {n}")); found.missing.push(n); }
            break;
        }
    }
    found.glibc = elves.values().filter_map(|e| e.glibc).max();
    // The folders holding libraries, for AppRun.
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    dirs.insert(PathBuf::from("usr/lib"));
    for (p, e) in &elves {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if e.is_lib || name.contains(".so") {
            if let Some(parent) = p.parent().and_then(|d| d.strip_prefix(appdir).ok()) {
                if !parent.as_os_str().is_empty() { dirs.insert(parent.to_path_buf()); }
            }
        }
    }
    found.lib_dirs = dirs.into_iter().collect();
    Ok(found)
}

fn rpm_arch(machine: u16) -> &'static str { if machine == 183 { "aarch64" } else { "x86_64" } }

/// For each library, the Fedora package that has it (dnf repoquery --whatprovides), downloaded (dnf download, as
/// you, nothing installed) and unpacked to take the library out. Returns (library, package, where it was put).
fn fedora_fetch(libs: &[String], machine: u16, libdir: &Path, log: &mut dyn FnMut(String)) -> Vec<(String, String, PathBuf)> {
    let mut got = Vec::new();
    if Command::new("dnf").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        log("dnf isn't here, so libraries this computer lacks can't be fetched".into());
        return got;
    }
    let suffix = if machine == 183 { "" } else { "()(64bit)" };
    let mut by_pkg: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for lib in libs {
        log(format!("Looking in Fedora for {lib}"));
        let o = Command::new("dnf")
            .args(["repoquery", "-q", "--latest-limit", "1", "--arch", rpm_arch(machine), "--queryformat", "%{name}\n", "--whatprovides"])
            .arg(format!("{lib}{suffix}"))
            .output();
        let pkg = o.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::trim).find(|l| !l.is_empty()).map(String::from));
        match pkg { Some(p) => by_pkg.entry(p).or_default().push(lib.clone()), None => log(format!("Fedora has no package with {lib}")) }
    }
    if by_pkg.is_empty() { return got; }
    let dl = std::env::temp_dir().join(format!("hearth-appimage-dnf-{}", std::process::id()));
    let _ = fs::create_dir_all(&dl);
    for (pkg, wanted) in by_pkg {
        log(format!("Downloading {pkg} from Fedora"));
        let ok = Command::new("dnf").args(["download", "-q", "--arch", rpm_arch(machine), "--destdir"]).arg(&dl).arg(&pkg)
            .status().map(|s| s.success()).unwrap_or(false);
        if !ok { log(format!("Couldn't download {pkg}")); continue; }
        let Some(rpm) = fs::read_dir(&dl).into_iter().flatten().flatten().map(|e| e.path())
            .find(|p| p.file_name().map(|n| { let n = n.to_string_lossy(); n.starts_with(&format!("{pkg}-")) && n.ends_with(".rpm") }).unwrap_or(false))
        else { continue };
        let stage = dl.join(format!("{pkg}.d"));
        let _ = fs::remove_dir_all(&stage);
        let mut out = match super::unpack::Out::new(&stage) { Ok(o) => o, Err(_) => continue };
        if let Err(e) = super::unpack::rpm(&rpm, &mut out) { log(format!("Couldn't unpack {pkg}: {e}")); continue; }
        for lib in wanted {
            let src = WalkDir::new(&stage).into_iter().flatten().map(|e| e.into_path())
                .find(|p| p.file_name().map(|n| n.to_string_lossy() == lib.as_str()).unwrap_or(false));
            if let Some(src) = src {
                // The link (libfoo.so.1) resolved to its file inside the package.
                let real = fs::canonicalize(&src).unwrap_or(src);
                let dst = libdir.join(&lib);
                if fs::copy(&real, &dst).is_ok() {
                    log(format!("Library from Fedora ({pkg}): {lib}"));
                    got.push((lib, pkg.clone(), dst));
                }
            }
        }
        let _ = fs::remove_dir_all(&stage);
        let _ = fs::remove_file(&rpm);
    }
    let _ = fs::remove_dir_all(&dl);
    got
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_system_is_left_out() {
        assert!(is_base("libc.so.6"));
        assert!(is_base("libGL.so.1"));
        assert!(is_base("libnvidia-glcore.so.570.1"));
        assert!(!is_base("libgtk-3.so.0"));
        assert!(!is_base("libyaml-0.so.2"));
    }

    #[test]
    fn glibc_versions() {
        assert_eq!(glibc_version("GLIBC_2.34"), Some((2, 34)));
        assert_eq!(glibc_version("GLIBC_2.2.5"), Some((2, 2)));
        assert_eq!(glibc_version("GLIBCXX_3.4"), None);
        assert!(Some((2, 39)) > Some((2, 4)));
    }

    #[test]
    fn reads_this_test_program() {
        let me = std::env::current_exe().unwrap();
        let e = read_elf(&me).expect("the test binary is ELF");
        assert!(e.needed.iter().any(|n| n.starts_with("libc.so")));
        assert!(e.glibc.is_some());
    }
}
