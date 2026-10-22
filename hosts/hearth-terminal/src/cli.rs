//! The command line, so other apps can open a terminal somewhere or run something in it:
//!   hearth-terminal [--cwd DIR] [-e CMD ARGS...]
//! Also understood: --working-directory DIR / --working-directory=DIR (foot's and others' spelling),
//! `-- CMD ARGS...`, -x (an old alias of -e), --help and --version.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Launch {
    /// The folder the first tab opens in.
    pub cwd: Option<String>,
    /// A program (and its arguments) to run in the first tab instead of the shell.
    pub command: Option<Vec<String>>,
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Run(Launch),
    Help,
    Version,
}

pub const USAGE: &str = "Hearth Terminal

Usage: hearth-terminal [--cwd DIR] [-e CMD ARGS...]

  --cwd DIR      open in this folder (also --working-directory DIR)
  -e CMD ARGS    run CMD in the terminal instead of your shell; everything after it is CMD's
  --help         show this
  --version      show the version
";

/// Reads the arguments (without the program name).
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Parsed, String> {
    let mut l = Launch::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "--cwd" | "--working-directory" | "-d" | "--dir" => {
                l.cwd = Some(it.next().ok_or_else(|| format!("{a} needs a folder"))?);
            }
            "-e" | "-x" | "--execute" | "--" => {
                let rest: Vec<String> = it.collect();
                if rest.is_empty() { return Err(format!("{a} needs a command")); }
                // `-e "ls -la"` (one word with spaces) is what some apps send: split it like a shell would.
                l.command = Some(if rest.len() == 1 && rest[0].contains(' ') { split_words(&rest[0]) } else { rest });
                break;
            }
            _ => {
                if let Some(d) = a.strip_prefix("--cwd=").or_else(|| a.strip_prefix("--working-directory=")) {
                    l.cwd = Some(d.to_string());
                } else if a.starts_with('-') {
                    return Err(format!("unknown option {a}"));
                } else if l.cwd.is_none() && std::path::Path::new(&a).is_dir() {
                    // A folder on its own (a file manager dropping one on the app): open there.
                    l.cwd = Some(a);
                } else {
                    return Err(format!("unexpected {a} (to run a command, put -e before it)"));
                }
            }
        }
    }
    Ok(Parsed::Run(l))
}

/// Splits "ls -la 'My Games'" into words: spaces separate, single and double quotes group, backslash escapes.
fn split_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut any = false;
    let mut quote: Option<char> = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => { if let Some(n) = chars.next() { cur.push(n); } }
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => { quote = Some(c); any = true; }
            (None, '\\') if !cfg!(windows) => { if let Some(n) = chars.next() { cur.push(n); any = true; } }
            (None, c) if c.is_whitespace() => { if any || !cur.is_empty() { out.push(std::mem::take(&mut cur)); any = false; } }
            (None, c) => { cur.push(c); any = true; }
        }
    }
    if any || !cur.is_empty() { out.push(cur); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(a: &[&str]) -> Result<Parsed, String> { parse(a.iter().map(|s| s.to_string())) }
    fn run(cwd: Option<&str>, cmd: Option<&[&str]>) -> Parsed {
        Parsed::Run(Launch { cwd: cwd.map(Into::into), command: cmd.map(|c| c.iter().map(|s| s.to_string()).collect()) })
    }

    #[test]
    fn nothing_means_a_shell_at_home() { assert_eq!(p(&[]), Ok(run(None, None))); }

    #[test]
    fn cwd_and_command() {
        assert_eq!(p(&["--cwd", "/tmp", "-e", "htop", "-d", "5"]), Ok(run(Some("/tmp"), Some(&["htop", "-d", "5"]))));
        assert_eq!(p(&["--working-directory=/srv", "--", "ls"]), Ok(run(Some("/srv"), Some(&["ls"]))));
    }

    #[test]
    fn one_word_command_is_split() {
        assert_eq!(p(&["-e", "ls -la 'My Games'"]), Ok(run(None, Some(&["ls", "-la", "My Games"]))));
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(p(&["--cwd"]).is_err());
        assert!(p(&["-e"]).is_err());
        assert!(p(&["--bogus"]).is_err());
        assert_eq!(p(&["--help"]), Ok(Parsed::Help));
    }
}
