//! Unpacking packages into a folder, safely: .deb (ar + tar), .rpm (lead, headers, compressed cpio), tarballs in
//! any compression, and .zip. Nothing in a package is run (no maintainer scripts, no rpm/dpkg), and nothing it holds
//! can land outside the folder: `..` and absolute paths are cleaned, and no file is written through a link that
//! came out of the package.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Component, Path, PathBuf};

pub fn err(msg: impl Into<String>) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, msg.into()) }

/// A path from an archive made relative and safe, or None (empty, or it climbs out with `..`).
pub fn clean_rel(p: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in Path::new(p).components() {
        match c {
            Component::Normal(s) => out.push(s),
            Component::CurDir | Component::RootDir => {}
            Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    if out.as_os_str().is_empty() { None } else { Some(out) }
}

/// Writes an archive's entries under `root`.
pub struct Out {
    pub root: PathBuf,
    pub files: usize,
    pub skipped: Vec<String>,
}

impl Out {
    pub fn new(root: &Path) -> io::Result<Out> {
        fs::create_dir_all(root)?;
        Ok(Out { root: root.to_path_buf(), files: 0, skipped: Vec::new() })
    }

    /// The parent folders of `rel` made (as real folders), refusing to go through a link: a package that ships
    /// `x -> /home/you` and then `x/.bashrc` can't write into your home.
    fn place(&self, rel: &Path) -> io::Result<PathBuf> {
        let mut cur = self.root.clone();
        if let Some(parent) = rel.parent() {
            for c in parent.components() {
                cur.push(c);
                match fs::symlink_metadata(&cur) {
                    Ok(m) if m.is_dir() => {}
                    Ok(m) if m.file_type().is_symlink() => {
                        // A link to a folder inside the package (lib -> usr/lib) is followed only when it stays inside.
                        let real = fs::canonicalize(&cur).map_err(|_| err(format!("{} goes through a broken link", rel.display())))?;
                        let root = fs::canonicalize(&self.root)?;
                        if !real.starts_with(&root) || !real.is_dir() {
                            return Err(err(format!("{} goes through a link out of the package", rel.display())));
                        }
                        cur = real;
                    }
                    Ok(_) => return Err(err(format!("{} is inside a file", rel.display()))),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        fs::create_dir(&cur)?;
                        fs::set_permissions(&cur, fs::Permissions::from_mode(0o755))?;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        let name = rel.file_name().ok_or_else(|| err("no file name"))?;
        let target = cur.join(name);
        if let Ok(m) = fs::symlink_metadata(&target) {
            if m.is_dir() && !m.file_type().is_symlink() { return Ok(target); }
            fs::remove_file(&target)?;
        }
        Ok(target)
    }

    pub fn file(&mut self, path: &str, data: &mut dyn Read, mode: u32) -> io::Result<()> {
        let Some(rel) = clean_rel(path) else { self.skipped.push(path.to_string()); return Ok(()) };
        let target = match self.place(&rel) { Ok(t) => t, Err(e) => { self.skipped.push(format!("{path}: {e}")); return Ok(()) } };
        if target.is_dir() { self.skipped.push(format!("{path}: a folder is there")); return Ok(()); }
        let mut f = fs::File::create(&target)?;
        io::copy(data, &mut f)?;
        // setuid/setgid/sticky never carried over; owner can always read and write (to clean up afterwards).
        fs::set_permissions(&target, fs::Permissions::from_mode((mode & 0o777) | 0o600))?;
        self.files += 1;
        Ok(())
    }

    pub fn dir(&mut self, path: &str, mode: u32) -> io::Result<()> {
        let Some(rel) = clean_rel(path) else { return Ok(()) };
        let target = match self.place(&rel) { Ok(t) => t, Err(e) => { self.skipped.push(format!("{path}: {e}")); return Ok(()) } };
        if !target.is_dir() { fs::create_dir(&target)?; }
        fs::set_permissions(&target, fs::Permissions::from_mode((mode & 0o777) | 0o700))?;
        Ok(())
    }

    /// Links are written as they are (never followed while unpacking); absolute ones are made relative later
    /// (appdir::fix_links) once everything is in place.
    pub fn link(&mut self, path: &str, to: &str) -> io::Result<()> {
        let Some(rel) = clean_rel(path) else { self.skipped.push(path.to_string()); return Ok(()) };
        let target = match self.place(&rel) { Ok(t) => t, Err(e) => { self.skipped.push(format!("{path}: {e}")); return Ok(()) } };
        if target.is_dir() { self.skipped.push(format!("{path}: a folder is there")); return Ok(()); }
        symlink(to, &target)?;
        Ok(())
    }

    /// A hard link: a copy of a file already unpacked.
    pub fn hardlink(&mut self, path: &str, existing: &str) -> io::Result<()> {
        let Some(src) = clean_rel(existing) else { return Ok(()) };
        let src = self.root.join(src);
        match fs::symlink_metadata(&src) {
            Ok(m) if m.is_file() => {
                let mode = m.permissions().mode();
                let mut f = fs::File::open(&src)?;
                self.file(path, &mut f, mode)
            }
            _ => { self.skipped.push(format!("{path}: hard link to {existing}, which isn't there")); Ok(()) }
        }
    }
}

/// A reader that undoes whatever compression it finds (gzip, xz, zstd, bzip2, lzma), or passes plain data through.
pub fn decompress<'a>(r: impl Read + 'a) -> io::Result<Box<dyn Read + 'a>> {
    let mut b = BufReader::with_capacity(1 << 16, r);
    let head = b.fill_buf()?.to_vec();
    let m = |sig: &[u8]| head.len() >= sig.len() && &head[..sig.len()] == sig;
    Ok(if m(&[0x1f, 0x8b]) {
        Box::new(flate2::read::MultiGzDecoder::new(b))
    } else if m(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
        Box::new(liblzma::read::XzDecoder::new_multi_decoder(b))
    } else if m(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Box::new(zstd::stream::read::Decoder::with_buffer(b)?)
    } else if m(b"BZh") {
        Box::new(bzip2::read::MultiBzDecoder::new(b))
    } else if m(&[0x5d, 0, 0]) {
        let stream = liblzma::stream::Stream::new_lzma_decoder(u64::MAX).map_err(|e| err(e.to_string()))?;
        Box::new(liblzma::read::XzDecoder::new_stream(b, stream))
    } else {
        Box::new(b)
    })
}

/// A tar stream (already decompressed or not) into `out`.
pub fn untar(r: impl Read, out: &mut Out) -> io::Result<()> {
    let mut a = tar::Archive::new(decompress(r)?);
    for e in a.entries()? {
        let mut e = e?;
        let path = e.path()?.to_string_lossy().into_owned();
        let mode = e.header().mode().unwrap_or(0o644);
        use tar::EntryType as T;
        match e.header().entry_type() {
            T::Regular | T::Continuous | T::GNUSparse => out.file(&path, &mut e, mode)?,
            T::Directory => out.dir(&path, mode)?,
            T::Symlink => {
                let to = e.link_name()?.map(|l| l.to_string_lossy().into_owned()).unwrap_or_default();
                out.link(&path, &to)?
            }
            T::Link => {
                let to = e.link_name()?.map(|l| l.to_string_lossy().into_owned()).unwrap_or_default();
                out.hardlink(&path, &to)?
            }
            _ => out.skipped.push(format!("{path}: device or special file")),
        }
    }
    Ok(())
}

pub fn unzip(path: &Path, out: &mut Out) -> io::Result<()> {
    let mut z = zip::ZipArchive::new(fs::File::open(path)?).map_err(|e| err(e.to_string()))?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| err(e.to_string()))?;
        let name = f.name().to_string();
        let mode = f.unix_mode().unwrap_or(if f.is_dir() { 0o755 } else { 0o644 });
        if f.is_dir() {
            out.dir(&name, mode)?;
        } else if mode & 0o170000 == 0o120000 {
            let mut to = String::new();
            f.read_to_string(&mut to)?;
            out.link(&name, &to)?;
        } else {
            out.file(&name, &mut f, mode)?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ .deb

/// What the package says about itself.
#[derive(Default, Debug, Clone)]
pub struct Meta {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub summary: String,
    pub depends: Vec<String>,
}

/// Debian's control file (`Key: value`, continuation lines start with a space).
pub fn parse_control(text: &str) -> Meta {
    let mut m = Meta::default();
    let mut key = String::new();
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') { continue; }
        if let Some((k, v)) = line.split_once(':') {
            key = k.trim().to_ascii_lowercase();
            let v = v.trim().to_string();
            match key.as_str() {
                "package" => m.name = v,
                "version" => m.version = v,
                "architecture" => m.arch = v,
                "description" => m.summary = v,
                "depends" | "pre-depends" => m.depends.extend(
                    v.split(',').map(|d| d.split('|').next().unwrap_or("").split_whitespace().next().unwrap_or("").to_string())
                        .filter(|d| !d.is_empty()),
                ),
                _ => {}
            }
        }
    }
    let _ = key;
    m
}

/// A .deb is an `ar` archive: debian-binary, control.tar.*, data.tar.*.
pub fn deb(path: &Path, out: &mut Out) -> io::Result<Meta> {
    let mut f = BufReader::new(fs::File::open(path)?);
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)?;
    if &magic != b"!<arch>\n" { return Err(err("not a .deb (no ar header)")); }
    let mut meta = Meta::default();
    let mut got_data = false;
    loop {
        let mut h = [0u8; 60];
        match f.read_exact(&mut h) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
        let name = String::from_utf8_lossy(&h[0..16]).trim().trim_end_matches('/').to_string();
        let size: u64 = String::from_utf8_lossy(&h[48..58]).trim().parse().map_err(|_| err("bad ar member size"))?;
        let mut member = (&mut f).take(size);
        if name.starts_with("control.tar") {
            let mut a = tar::Archive::new(decompress(&mut member)?);
            for e in a.entries()? {
                let mut e = e?;
                let p = e.path()?.to_string_lossy().into_owned();
                if p.trim_start_matches("./") == "control" {
                    let mut s = String::new();
                    e.read_to_string(&mut s)?;
                    meta = parse_control(&s);
                }
            }
        } else if name.starts_with("data.tar") {
            untar(&mut member, out)?;
            got_data = true;
        }
        io::copy(&mut member, &mut io::sink())?;
        if size % 2 == 1 { let mut pad = [0u8; 1]; let _ = f.read_exact(&mut pad); }
    }
    if !got_data { return Err(err("the .deb has no data.tar")); }
    Ok(meta)
}

// ------------------------------------------------------------------------------------------------ .rpm

fn be32(b: &[u8]) -> u32 { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) }

/// One rpm header section: (tag → value as text) for the string tags asked for.
fn read_header(r: &mut impl Read, pad8: bool) -> io::Result<HashMap<u32, String>> {
    let mut h = [0u8; 16];
    r.read_exact(&mut h)?;
    if h[0..3] != [0x8e, 0xad, 0xe8] { return Err(err("not an rpm header")); }
    let n = be32(&h[8..12]) as usize;
    let size = be32(&h[12..16]) as usize;
    if n > 100_000 || size > 256 << 20 { return Err(err("rpm header too big")); }
    let mut index = vec![0u8; n * 16];
    r.read_exact(&mut index)?;
    let mut store = vec![0u8; size];
    r.read_exact(&mut store)?;
    if pad8 {
        let pad = (8 - (16 + n * 16 + size) % 8) % 8;
        let mut p = vec![0u8; pad];
        r.read_exact(&mut p)?;
    }
    let mut tags = HashMap::new();
    for e in index.chunks(16) {
        let (tag, ty, off) = (be32(&e[0..4]), be32(&e[4..8]), be32(&e[8..12]) as usize);
        // 6 string, 8 string array, 9 i18n string: the first string.
        if matches!(ty, 6 | 8 | 9) && off < store.len() {
            let s = &store[off..];
            let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
            tags.insert(tag, String::from_utf8_lossy(&s[..end]).into_owned());
        }
        // 4 int32: kept as text too (unused for now, but cheap).
        if ty == 4 && off + 4 <= store.len() { tags.entry(tag).or_insert_with(|| be32(&store[off..off + 4]).to_string()); }
    }
    Ok(tags)
}

/// An .rpm: 96-byte lead, the signature header (padded to 8), the main header, then the compressed cpio payload.
pub fn rpm(path: &Path, out: &mut Out) -> io::Result<Meta> {
    let mut f = BufReader::new(fs::File::open(path)?);
    let mut lead = [0u8; 96];
    f.read_exact(&mut lead)?;
    if lead[0..4] != [0xed, 0xab, 0xee, 0xdb] { return Err(err("not an .rpm")); }
    read_header(&mut f, true)?;
    let t = read_header(&mut f, false)?;
    let g = |k: u32| t.get(&k).cloned().unwrap_or_default();
    let release = g(1002);
    let meta = Meta {
        name: g(1000),
        version: if release.is_empty() { g(1001) } else { format!("{}-{}", g(1001), release) },
        arch: g(1022),
        summary: g(1004),
        depends: Vec::new(),
    };
    cpio(decompress(f)?, out)?;
    Ok(meta)
}

fn hex(b: &[u8]) -> io::Result<u64> {
    u64::from_str_radix(std::str::from_utf8(b).map_err(|_| err("bad cpio header"))?, 16).map_err(|_| err("bad cpio header"))
}

/// cpio "newc" (070701/070702), what rpm payloads are. Hard links carry their data on the last entry of the group.
pub fn cpio(mut r: impl Read, out: &mut Out) -> io::Result<()> {
    let mut pending: HashMap<u64, Vec<String>> = HashMap::new();
    loop {
        let mut h = [0u8; 110];
        r.read_exact(&mut h)?;
        if &h[0..6] != b"070701" && &h[0..6] != b"070702" { return Err(err("not a newc cpio archive")); }
        let f = |i: usize| hex(&h[6 + i * 8..14 + i * 8]);
        let (ino, mode, nlink, size, namesize) = (f(0)?, f(1)? as u32, f(4)?, f(6)?, f(11)? as usize);
        if namesize > 65536 { return Err(err("cpio name too long")); }
        let mut name = vec![0u8; namesize];
        r.read_exact(&mut name)?;
        let pad = (4 - (110 + namesize) % 4) % 4;
        io::copy(&mut (&mut r).take(pad as u64), &mut io::sink())?;
        let name = String::from_utf8_lossy(&name[..namesize.saturating_sub(1)]).into_owned();
        if name == "TRAILER!!!" { break; }
        let mut data = (&mut r).take(size);
        match mode & 0o170000 {
            0o040000 => out.dir(&name, mode)?,
            0o120000 => {
                let mut to = String::new();
                data.read_to_string(&mut to)?;
                out.link(&name, &to)?;
            }
            0o100000 => {
                if nlink > 1 && size == 0 {
                    pending.entry(ino).or_default().push(name);
                } else {
                    out.file(&name, &mut data, mode)?;
                    for other in pending.remove(&ino).unwrap_or_default() { out.hardlink(&other, &name)?; }
                }
            }
            _ => out.skipped.push(format!("{name}: device or special file")),
        }
        io::copy(&mut data, &mut io::sink())?;
        let dpad = (4 - size % 4) % 4;
        io::copy(&mut (&mut r).take(dpad), &mut io::sink())?;
    }
    // Hard links whose data never came: empty files, as cpio itself makes them.
    for (_, names) in pending { for n in names { out.file(&n, &mut io::empty(), 0o644)?; } }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ kinds

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind { Deb, Rpm, Tar, Zip }

/// What a file is, by its contents first and its name second.
pub fn kind(path: &Path) -> Option<Kind> {
    let mut head = [0u8; 8];
    let n = fs::File::open(path).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    let head = &head[..n];
    if head.starts_with(b"!<arch>\n") { return Some(Kind::Deb); }
    if head.starts_with(&[0xed, 0xab, 0xee, 0xdb]) { return Some(Kind::Rpm); }
    if head.starts_with(b"PK\x03\x04") { return Some(Kind::Zip); }
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if [".tar", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.zst", ".tzst", ".tar.bz2", ".tbz2", ".tbz"].iter().any(|e| name.ends_with(e)) {
        return Some(Kind::Tar);
    }
    if head.starts_with(&[0x1f, 0x8b]) || head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z']) || head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) || head.starts_with(b"BZh") {
        return Some(Kind::Tar);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hearth-appimage-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn paths_are_cleaned() {
        assert_eq!(clean_rel("./usr/bin/x"), Some(PathBuf::from("usr/bin/x")));
        assert_eq!(clean_rel("/opt/a"), Some(PathBuf::from("opt/a")));
        assert_eq!(clean_rel("../../etc/passwd"), None);
        assert_eq!(clean_rel("usr/../../x"), None);
        assert_eq!(clean_rel("./"), None);
    }

    #[test]
    fn nothing_is_written_through_a_link_out_of_the_package() {
        let d = tmp("link");
        let outside = tmp("outside");
        fs::create_dir_all(&outside).unwrap();
        let mut o = Out::new(&d).unwrap();
        o.link("x", outside.to_str().unwrap()).unwrap();
        o.file("x/evil", &mut &b"no"[..], 0o644).unwrap();
        assert!(!outside.join("evil").exists());
        assert_eq!(o.skipped.len(), 1);
        // A link inside the package is fine.
        o.dir("usr/lib", 0o755).unwrap();
        o.link("lib", "usr/lib").unwrap();
        o.file("lib/libok.so", &mut &b"ok"[..], 0o644).unwrap();
        assert!(d.join("usr/lib/libok.so").exists());
        let _ = fs::remove_dir_all(&d);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn setuid_is_dropped() {
        let d = tmp("mode");
        let mut o = Out::new(&d).unwrap();
        o.file("bin/x", &mut &b"#!/bin/sh"[..], 0o4755).unwrap();
        assert_eq!(fs::metadata(d.join("bin/x")).unwrap().permissions().mode() & 0o7777, 0o755);
        let _ = fs::remove_dir_all(&d);
    }

    fn newc(name: &str, mode: u32, ino: u64, nlink: u64, data: &[u8]) -> Vec<u8> {
        let mut v = format!("070701{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
            ino, mode, 0, 0, nlink, 0, data.len(), 0, 0, 0, 0, name.len() + 1, 0).into_bytes();
        v.extend_from_slice(name.as_bytes());
        v.push(0);
        while v.len() % 4 != 0 { v.push(0); }
        v.extend_from_slice(data);
        while v.len() % 4 != 0 { v.push(0); }
        v
    }

    #[test]
    fn cpio_files_links_and_hard_links() {
        let d = tmp("cpio");
        let mut a = Vec::new();
        a.extend(newc("./usr/bin", 0o040755, 1, 2, b""));
        a.extend(newc("./usr/bin/hello", 0o100755, 2, 1, b"#!/bin/sh\necho hi\n"));
        a.extend(newc("./usr/bin/hi", 0o120777, 3, 1, b"hello"));
        a.extend(newc("./usr/bin/a", 0o100644, 4, 2, b""));
        a.extend(newc("./usr/bin/b", 0o100644, 4, 2, b"shared"));
        a.extend(newc("TRAILER!!!", 0, 0, 1, b""));
        let mut o = Out::new(&d).unwrap();
        cpio(&a[..], &mut o).unwrap();
        assert_eq!(fs::read_to_string(d.join("usr/bin/hello")).unwrap(), "#!/bin/sh\necho hi\n");
        assert_eq!(fs::read_link(d.join("usr/bin/hi")).unwrap(), PathBuf::from("hello"));
        assert_eq!(fs::read_to_string(d.join("usr/bin/a")).unwrap(), "shared");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn control_file() {
        let m = parse_control("Package: demo\nVersion: 1.2-3\nArchitecture: amd64\nDepends: libc6 (>= 2.34), libgtk-3-0 | libgtk-4-1, python3\nDescription: A demo\n Long text\n");
        assert_eq!((m.name.as_str(), m.version.as_str(), m.arch.as_str(), m.summary.as_str()), ("demo", "1.2-3", "amd64", "A demo"));
        assert_eq!(m.depends, vec!["libc6", "libgtk-3-0", "python3"]);
    }
}
