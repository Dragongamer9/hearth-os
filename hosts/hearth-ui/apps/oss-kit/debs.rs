//! Installing what an app is built on from its Hearth Store copy alone (the owner, Nov 26: "All this has to be updated
//! thru the store"; Hearth OS on Debian is writable): Debian trixie's own .deb, by its exact version and SHA-256 (the
//! pins below, read from Debian's signed package lists by CI), downloaded from deb.debian.org into Hearth's cache,
//! checked, and installed with `hearth-admin deb-install` (it asks for the administrator's password; apt brings any
//! missing dependencies from Debian; hearth-unlock keeps it across Hearth OS updates, device rules and services
//! included). Or a small .deb Hearth makes itself (a device rule), built with dpkg-deb and installed the same way.
//! Nothing here starts a service: each app turns on only what's needed, when the person turns it on.
#![allow(dead_code)]

use std::path::PathBuf;

pub const ADMIN: &str = "/usr/libexec/nimbus/hearth-admin";
pub const MIRROR: &str = "https://deb.debian.org/debian";

/// One of Debian's packages, exactly: its file in Debian's pool and that file's SHA-256.
#[derive(Debug, Clone, Copy)]
pub struct Pin {
    pub package: &'static str,
    pub version: &'static str,
    pub pool: &'static str,
    pub sha256: &'static str,
}

fn cache() -> PathBuf {
    let h = hearth_ui::paths::home();
    std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".cache")).join("hearth/debs")
}

fn sha256_of(f: &std::path::Path) -> String {
    let o = hearth_ui::run::blocking(&["sha256sum".to_string(), f.to_string_lossy().to_string()]);
    o.out.split_whitespace().next().unwrap_or("").to_lowercase()
}

/// The pinned .deb in Hearth's cache, downloaded when it isn't there, refused when it isn't the file Debian made.
pub fn fetch(p: &Pin) -> Result<PathBuf, String> {
    if p.sha256.len() != 64 {
        return Err(format!("{} has no pinned file yet", p.package));
    }
    let d = cache();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let name = p.pool.rsplit('/').next().unwrap_or(p.package).to_string();
    let f = d.join(&name);
    if f.is_file() && sha256_of(&f) == p.sha256 {
        return Ok(f);
    }
    let part = d.join(format!("{name}.part"));
    let o = hearth_ui::run::blocking(&["curl".to_string(), "-fsSL".into(), "--max-time".into(), "900".into(), "-o".into(), part.to_string_lossy().to_string(), format!("{MIRROR}/{}", p.pool)]);
    if !o.ok {
        let _ = std::fs::remove_file(&part);
        return Err(format!("Debian's {} couldn't be downloaded: {}", p.package, o.why()));
    }
    if sha256_of(&part) != p.sha256 {
        let _ = std::fs::remove_file(&part);
        return Err(format!("the downloaded {} isn't the file Debian made (its SHA-256 doesn't match), so it wasn't installed", p.package));
    }
    std::fs::rename(&part, &f).map_err(|e| e.to_string())?;
    Ok(f)
}

/// A .deb on the system (asks for the administrator's password).
pub fn install(deb: &std::path::Path) -> Result<(), String> {
    if !std::path::Path::new(ADMIN).exists() {
        return Err("this PC's Hearth OS can't install packages (hearth-admin isn't here)".into());
    }
    let o = hearth_ui::run::blocking(&["pkexec".to_string(), ADMIN.into(), "deb-install".into(), deb.to_string_lossy().to_string()]);
    if o.ok {
        Ok(())
    } else if o.code == 126 || o.code == 127 {
        Err("the administrator's password wasn't given".into())
    } else {
        Err(o.why())
    }
}

/// Downloads and installs each pinned package, in order.
pub fn get(pins: &[Pin]) -> Result<(), String> {
    let mut files = Vec::new();
    for p in pins {
        files.push(fetch(p)?);
    }
    for f in &files {
        install(f)?;
    }
    Ok(())
}

/// A small package Hearth makes itself: `files` are (path on the system, contents, mode); `postinst` runs after it's
/// in (as dpkg runs it). Built with Debian's own dpkg-deb.
pub fn build(package: &str, version: &str, about: &str, files: &[(&str, String, u32)], postinst: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = cache().join(format!("{package}-build"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("DEBIAN")).map_err(|e| e.to_string())?;
    let control = format!("Package: {package}\nVersion: {version}\nArchitecture: all\nMaintainer: Hearth OS <noreply@hearthos.fun>\nSection: admin\nPriority: optional\nDescription: {about}\n");
    std::fs::write(root.join("DEBIAN/control"), control).map_err(|e| e.to_string())?;
    if !postinst.is_empty() {
        let pi = root.join("DEBIAN/postinst");
        std::fs::write(&pi, format!("#!/bin/sh\nset -e\n{postinst}\nexit 0\n")).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&pi, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    for (path, body, mode) in files {
        let f = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(f.parent().unwrap_or(&root)).map_err(|e| e.to_string())?;
        std::fs::write(&f, body).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(*mode)).map_err(|e| e.to_string())?;
    }
    let out = cache().join(format!("{package}_{version}_all.deb"));
    let o = hearth_ui::run::blocking(&["dpkg-deb".to_string(), "--root-owner-group".into(), "--build".into(), root.to_string_lossy().to_string(), out.to_string_lossy().to_string()]);
    let _ = std::fs::remove_dir_all(&root);
    if o.ok { Ok(out) } else { Err(format!("the package couldn't be made: {}", o.why())) }
}
