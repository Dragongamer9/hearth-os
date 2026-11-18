//! One conversion: a package in, an AppImage out, with a report of what was done and what might not work.

pub mod appdir;
pub mod libs;
pub mod menu;
pub mod pack;
pub mod unpack;

use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use unpack::Kind;

#[derive(Debug, Clone)]
pub struct Options {
    /// Where the AppImage goes.
    pub out_dir: PathBuf,
    /// Libraries this computer lacks fetched from Fedora.
    pub fetch: bool,
    /// Bundle libraries at all (off: only the package's own files).
    pub bundle: bool,
    /// Put it in the app menu when done.
    pub menu: bool,
}

/// What happened, for the window and the command line.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Report {
    pub input: String,
    pub output: String,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub kind: String,
    pub size: u64,
    pub libs_from_host: Vec<String>,
    pub libs_from_fedora: Vec<String>,
    pub libs_missing: Vec<String>,
    pub warnings: Vec<String>,
    pub menu_entry: String,
    /// The AppImage's own menu entry (for adding it to the menu later).
    pub desktop: String,
    pub id: String,
}

/// Progress, as it happens.
pub enum Event { Step(String), Log(String) }

fn workdir() -> io::Result<PathBuf> {
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("hearth-appimage").join("work");
    fs::create_dir_all(&base)?;
    let d = base.join(format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
    fs::create_dir_all(&d)?;
    Ok(d)
}

/// The package's name from its file name (for tarballs and zips): `Foo-1.2.3-linux-x64.tar.gz` → (Foo, 1.2.3).
pub fn name_from_file(path: &Path) -> (String, String) {
    let mut n = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    for ext in [".tar.gz", ".tar.xz", ".tar.zst", ".tar.bz2", ".tgz", ".txz", ".tzst", ".tbz2", ".tbz", ".tar", ".zip", ".deb", ".rpm"] {
        if n.to_ascii_lowercase().ends_with(ext) { n.truncate(n.len() - ext.len()); break; }
    }
    let parts: Vec<&str> = n.split(['-', '_']).collect();
    let vi = parts.iter().position(|p| p.trim_start_matches('v').chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && p.contains('.'));
    match vi {
        Some(i) if i > 0 => (parts[..i].join("-"), parts[i].trim_start_matches('v').to_string()),
        _ => (n.clone(), String::new()),
    }
}

/// When a tarball is one folder (Foo-1.2/...), that folder is the app; a tarball laid out like / (usr/bin/...) stays.
fn tarball_root(staged: &Path, appdir: &Path, id: &str) -> io::Result<()> {
    let entries: Vec<_> = fs::read_dir(staged)?.flatten().collect();
    let looks_like_root = entries.iter().any(|e| { let n = e.file_name(); n == "usr" || n == "opt" });
    if looks_like_root {
        fs::rename(staged, appdir)?;
        return Ok(());
    }
    let inner = if entries.len() == 1 && entries[0].path().is_dir() && !entries[0].file_type().map(|t| t.is_symlink()).unwrap_or(true) {
        entries[0].path()
    } else {
        staged.to_path_buf()
    };
    fs::create_dir_all(appdir.join("opt"))?;
    fs::rename(&inner, appdir.join("opt").join(id))?;
    Ok(())
}

pub fn convert(input: &Path, opt: &Options, on: &mut dyn FnMut(Event)) -> io::Result<Report> {
    let mut r = Report { input: input.display().to_string(), ..Default::default() };
    let kind = unpack::kind(input).ok_or_else(|| unpack::err("this isn't a .deb, .rpm, tarball or .zip"))?;
    r.kind = format!("{kind:?}").to_lowercase();
    let work = workdir()?;
    let result = run(input, kind, opt, &work, &mut r, on);
    let _ = fs::remove_dir_all(&work);
    result.map(|_| r)
}

fn run(input: &Path, kind: Kind, opt: &Options, work: &Path, r: &mut Report, on: &mut dyn FnMut(Event)) -> io::Result<()> {
    let appdir = work.join("AppDir");
    on(Event::Step("Unpacking".into()));
    let (fname, fver) = name_from_file(input);
    let meta = match kind {
        Kind::Deb => { let mut o = unpack::Out::new(&appdir)?; let m = unpack::deb(input, &mut o)?; note_skipped(&o, r); m }
        Kind::Rpm => { let mut o = unpack::Out::new(&appdir)?; let m = unpack::rpm(input, &mut o)?; note_skipped(&o, r); m }
        Kind::Tar | Kind::Zip => {
            let staged = work.join("staged");
            let mut o = unpack::Out::new(&staged)?;
            if kind == Kind::Tar { unpack::untar(fs::File::open(input)?, &mut o)?; } else { unpack::unzip(input, &mut o)?; }
            note_skipped(&o, r);
            tarball_root(&staged, &appdir, &appdir::slug(&fname))?;
            unpack::Meta { name: fname.clone(), version: fver.clone(), ..Default::default() }
        }
    };
    let name_hint = if meta.name.is_empty() { fname.clone() } else { meta.name.clone() };
    on(Event::Log(format!("{} {} ({})", name_hint, meta.version, r.kind)));

    // Machine type the package is for.
    let arch = meta.arch.to_ascii_lowercase();
    let host_arch = std::env::consts::ARCH;
    let pkg_arch = match arch.as_str() { "amd64" | "x86_64" => "x86_64", "arm64" | "aarch64" => "aarch64", "i386" | "i686" => "x86", "all" | "noarch" | "" => "", o => o };
    if !pkg_arch.is_empty() && pkg_arch != host_arch {
        r.warnings.push(format!("This package is for {pkg_arch}, and this computer is {host_arch}: the AppImage will only run on {pkg_arch}."));
    }

    on(Event::Step("Fixing paths".into()));
    let links = appdir::fix_links(&appdir);
    let scripts = appdir::fix_scripts(&appdir);
    on(Event::Log(format!("{links} links and {scripts} paths in launcher scripts pointed inside the AppImage")));

    // Menu entry, program, icon.
    on(Event::Step("Finding the app".into()));
    let desktops = appdir::find_desktops(&appdir, &name_hint);
    let (mut desktop, program, args) = match desktops.first() {
        Some((_, d)) => {
            let (prog, args) = appdir::exec_program(d.get("Exec").unwrap_or(""));
            let found = appdir::find_program(&appdir, &prog);
            if found.is_none() { r.warnings.push(format!("The menu entry starts \"{prog}\", which isn't in the package; guessed the program instead.")); }
            (d.clone(), found, args)
        }
        None => (appdir::Desktop::default(), None, String::new()),
    };
    let program = match program.or_else(|| appdir::guess_program(&appdir, &name_hint)) {
        Some(p) => p,
        None => return Err(unpack::err("couldn't find a program to start in this package")),
    };
    // A link (usr/bin/foo -> ../../opt/Foo/foo) started through its target, so the app finds its files beside it.
    let program = match fs::canonicalize(&program) {
        Ok(real) if real.starts_with(fs::canonicalize(&appdir)?) => real.strip_prefix(fs::canonicalize(&appdir)?).map(|p| appdir.join(p)).unwrap_or(program),
        _ => program,
    };
    on(Event::Log(format!("Starts: {}", program.strip_prefix(&appdir).unwrap_or(&program).display())));
    if !libs::is_elf(&program) {
        let head = fs::read(&program).map(|b| String::from_utf8_lossy(&b[..b.len().min(200)]).into_owned()).unwrap_or_default();
        let first = head.lines().next().unwrap_or("");
        for (needle, what) in [("python", "Python"), ("java", "Java"), ("node", "Node.js"), ("perl", "Perl"), ("ruby", "Ruby")] {
            if first.contains(needle) {
                r.warnings.push(format!("This app runs on {what}, which isn't put inside: it uses this computer's {what} and its modules."));
            }
        }
    }
    let script_deps: Vec<&String> = meta.depends.iter().filter(|d| d.starts_with("python3-") || d.starts_with("default-jre") || d.starts_with("openjdk") || d.starts_with("node-") || d.starts_with("lib") && d.ends_with("-perl")).collect();
    if !script_deps.is_empty() {
        r.warnings.push(format!("It needs these, which aren't libraries and aren't put inside: {}.", script_deps.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }
    for d in &meta.depends {
        if ["systemd", "dkms", "polkitd", "policykit-1", "linux-headers", "udev"].iter().any(|x| d.starts_with(x)) {
            r.warnings.push(format!("It depends on {d}: parts that need services or drivers set up won't work from an AppImage."));
            break;
        }
    }

    let display_name = desktop.get("Name").map(String::from).unwrap_or_else(|| name_hint.clone());
    let id = appdir::slug(&name_hint);
    r.name = display_name.clone();
    r.version = if meta.version.is_empty() { fver.clone() } else { meta.version.clone() };

    // Libraries.
    let mut machine = libs::read_elf(&program).map(|e| e.machine).unwrap_or(0);
    let mut lib_dirs = vec![PathBuf::from("usr/lib")];
    if opt.bundle {
        on(Event::Step("Gathering libraries".into()));
        let mut log = |s: String| on(Event::Log(s));
        let f = libs::bundle(&appdir, opt.fetch, &mut log)?;
        if machine == 0 { machine = f.machine; }
        lib_dirs = f.lib_dirs;
        r.libs_from_host = f.from_host;
        r.libs_from_fedora = f.from_fedora.into_iter().map(|(l, p)| format!("{l} ({p})")).collect();
        r.libs_missing = f.missing.clone();
        if !f.missing.is_empty() {
            r.warnings.push(format!("These libraries weren't found anywhere, so it may not start: {}.", f.missing.join(", ")));
        }
        if let (Some(need), Some(have)) = (f.glibc, libs::host_glibc()) {
            if need > have {
                r.warnings.push(format!("It needs glibc {}.{}, newer than this computer's {}.{}: it won't start here until the system is newer.", need.0, need.1, have.0, have.1));
            }
            on(Event::Log(format!("Needs glibc {}.{} (this computer has {}.{})", need.0, need.1, have.0, have.1)));
        }
    }
    if machine == 0 { machine = if host_arch == "aarch64" { 183 } else { 62 }; }

    // AppRun, the menu entry and icon at the top.
    on(Event::Step("Writing the AppImage's launcher".into()));
    appdir::write_apprun(&appdir, &program, &args, &lib_dirs)?;
    let icon_name = desktop.get("Icon").unwrap_or("").to_string();
    let icon_src = appdir::find_icon(&appdir, &icon_name)
        .or_else(|| appdir::find_icon(&appdir, &name_hint))
        .or_else(|| appdir::find_icon(&appdir, &id));
    let icon_file = match &icon_src {
        Some(src) => {
            let ext = src.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "png".into());
            let dst = appdir.join(format!("{id}.{ext}"));
            fs::copy(src, &dst)?;
            dst
        }
        None => {
            r.warnings.push("The package has no icon, so it gets Hearth's plain package icon.".into());
            let dst = appdir.join(format!("{id}.svg"));
            fs::write(&dst, appdir::FALLBACK_ICON)?;
            dst
        }
    };
    let _ = fs::remove_file(appdir.join(".DirIcon"));
    std::os::unix::fs::symlink(icon_file.file_name().unwrap(), appdir.join(".DirIcon"))?;
    desktop.set("Type", "Application");
    desktop.set("Name", &display_name);
    let prog_name = program.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| id.clone());
    let exec_args = appdir::exec_program(desktop.get("Exec").unwrap_or("")).1;
    let field = ["%U", "%u", "%F", "%f"].iter().find(|f| exec_args.contains(**f)).map(|f| format!(" {f}")).unwrap_or_default();
    desktop.set("Exec", &format!("{prog_name}{field}"));
    desktop.set("Icon", &id);
    desktop.remove("TryExec");
    desktop.remove("Actions"); // their [Desktop Action] groups aren't carried over
    desktop.remove("DBusActivatable");
    if desktop.get("Categories").map(|c| c.trim().is_empty()).unwrap_or(true) { desktop.set("Categories", "Utility;"); }
    if desktop.get("Comment").is_none() && !meta.summary.is_empty() { desktop.set("Comment", &meta.summary); }
    desktop.set("X-AppImage-Version", &r.version);
    desktop.set("X-Hearth-Made-From", input.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().as_str());
    fs::write(appdir.join(format!("{id}.desktop")), desktop.text())?;
    r.desktop = desktop.text();
    r.id = id.clone();
    r.icon = icon_file.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();

    // Pack.
    on(Event::Step("Packing".into()));
    let mut log = |s: String| on(Event::Log(s));
    let rt = pack::runtime(machine, &mut log)?;
    let ver = if r.version.is_empty() { String::new() } else { format!("-{}", appdir::slug(&r.version)) };
    let file = format!("{}{}-{}.AppImage", display_name.replace(['/', ' '], "_"), ver, pack::arch_name(machine));
    let out = opt.out_dir.join(file);
    let sizes = appdir::sizes(&appdir);
    on(Event::Log(format!("{} files, {} MB before packing", sizes.get("files").unwrap_or(&0), sizes.get("bytes").unwrap_or(&0) / 1_000_000)));
    // The icon kept for the menu (the AppDir goes away).
    let keep_icon = dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("hearth-appimage/icons").join(icon_file.file_name().unwrap());
    if let Some(d) = keep_icon.parent() { let _ = fs::create_dir_all(d); }
    let _ = fs::copy(&icon_file, &keep_icon);
    pack::appimage(&appdir, &rt, &out)?;
    r.output = out.display().to_string();
    r.size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    r.icon = keep_icon.display().to_string();
    on(Event::Log(format!("Made {} ({} MB)", out.display(), r.size / 1_000_000)));

    if opt.menu {
        on(Event::Step("Adding it to the app menu".into()));
        match menu::add(&out, &desktop, &id, &keep_icon) {
            Ok(p) => r.menu_entry = p.display().to_string(),
            Err(e) => r.warnings.push(format!("Couldn't add it to the app menu: {e}")),
        }
    }
    Ok(())
}

fn note_skipped(o: &unpack::Out, r: &mut Report) {
    if !o.skipped.is_empty() {
        r.warnings.push(format!("{} entries in the package were left out (unsafe paths or special files), e.g. {}", o.skipped.len(), o.skipped[0]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_files() {
        assert_eq!(name_from_file(Path::new("/x/Foo-1.2.3-linux-x64.tar.gz")), ("Foo".into(), "1.2.3".into()));
        assert_eq!(name_from_file(Path::new("my_app-v2.0.tar.xz")), ("my-app".into(), "2.0".into()));
        assert_eq!(name_from_file(Path::new("tool.zip")), ("tool".into(), "".into()));
    }
}
