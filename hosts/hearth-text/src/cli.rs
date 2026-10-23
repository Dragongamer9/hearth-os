//! The command line, so Hearth Files and other apps can open files in Hearth Text:
//!   hearth-text [--readonly] [--line N | +N] [FILE[:LINE]...]
//! Also: --new (an empty tab even with files), --help, --version, and file:// URLs (what a .desktop %U passes).

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Launch {
    /// Files to open, each in its own tab.
    pub files: Vec<String>,
    /// Open them read-only (Hearth Files' "view" for files that aren't text, or that you can't change).
    pub readonly: bool,
    /// The line to go to in the first file.
    pub line: Option<u32>,
    /// An empty tab as well.
    pub new: bool,
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Run(Launch),
    Help,
    Version,
}

pub const USAGE: &str = "Hearth Text

Usage: hearth-text [--readonly] [--line N] [FILE...]

  FILE           open it (FILE:N goes to line N if there's no file called that)
  --readonly     open the files read-only (also --view)
  --line N, +N   go to line N in the first file
  --new          open an empty tab too
  --help         show this
  --version      show the version
";

/// "file:///home/me/My%20Notes.txt" → "/home/me/My Notes.txt"; anything else stays as it is.
pub fn from_uri(s: &str) -> String {
    let Some(rest) = s.strip_prefix("file://") else { return s.to_string() };
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    let p = String::from_utf8_lossy(&out).into_owned();
    // Windows: file:///C:/x → C:/x
    if cfg!(windows) && p.len() > 2 && p.as_bytes()[0] == b'/' && p.as_bytes()[2] == b':' { p[1..].to_string() } else { p }
}

/// Reads the arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Parsed, String> {
    let mut l = Launch::default();
    let mut it = args.into_iter();
    let mut only_files = false;
    while let Some(a) = it.next() {
        if only_files { l.files.push(from_uri(&a)); continue; }
        match a.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "-r" | "--readonly" | "--read-only" | "--view" => l.readonly = true,
            "--new" | "-n" => l.new = true,
            "--" => only_files = true,
            "-l" | "--line" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a line number"))?;
                l.line = Some(v.parse().map_err(|_| format!("{v} isn't a line number"))?);
            }
            _ => {
                if let Some(n) = a.strip_prefix("--line=") {
                    l.line = Some(n.parse().map_err(|_| format!("{n} isn't a line number"))?);
                } else if let Some(n) = a.strip_prefix('+').filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) {
                    l.line = n.parse().ok();
                } else if a.starts_with('-') && a.len() > 1 {
                    return Err(format!("unknown option {a}"));
                } else {
                    let f = from_uri(&a);
                    // notes.txt:12 (compilers and grep print this) when there's no file with that whole name.
                    if !std::path::Path::new(&f).exists() {
                        if let Some((file, n)) = f.rsplit_once(':') {
                            if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !file.is_empty() && !file.ends_with(':') {
                                if l.line.is_none() && l.files.is_empty() { l.line = n.parse().ok(); }
                                l.files.push(file.to_string());
                                continue;
                            }
                        }
                    }
                    l.files.push(f);
                }
            }
        }
    }
    Ok(Parsed::Run(l))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(a: &[&str]) -> Result<Parsed, String> { parse(a.iter().map(|s| s.to_string())) }
    fn files(a: &[&str]) -> Launch { match p(a).unwrap() { Parsed::Run(l) => l, other => panic!("{other:?}") } }

    #[test]
    fn nothing_means_an_empty_tab() { assert_eq!(files(&[]), Launch::default()); }

    #[test]
    fn files_and_options() {
        let l = files(&["--readonly", "/etc/fstab", "+3", "b.txt"]);
        assert_eq!(l.files, vec!["/etc/fstab", "b.txt"]);
        assert!(l.readonly);
        assert_eq!(l.line, Some(3));
        assert_eq!(files(&["--line", "7", "x"]).line, Some(7));
        assert_eq!(files(&["--", "-odd-name"]).files, vec!["-odd-name"]);
    }

    #[test]
    fn uris_and_line_suffixes() {
        assert_eq!(files(&["file:///tmp/My%20Notes.txt"]).files, vec!["/tmp/My Notes.txt"]);
        let l = files(&["/no/such/dir/main.rs:42"]);
        assert_eq!((l.files, l.line), (vec!["/no/such/dir/main.rs".to_string()], Some(42)));
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(p(&["--line"]).is_err());
        assert!(p(&["--line", "x"]).is_err());
        assert!(p(&["--bogus"]).is_err());
        assert_eq!(p(&["--help"]), Ok(Parsed::Help));
    }
}
