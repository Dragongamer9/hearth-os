//! AppImage Maker (hearth-appimage): Hearth OS's tool that turns apps packaged for other systems (.deb, .rpm,
//! tarballs, .zip) into AppImages that run here as one file, with their libraries inside.
//!
//!   hearth-appimage                         the window (drop packages on it)
//!   hearth-appimage FILE...                 the window, with those files already converting
//!   hearth-appimage convert [options] FILE  on the command line (also in the build without the window)
//!
//! The work is in convert/ (unpack, libraries, AppDir, pack); the window is gui.rs and ui/.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod convert;
#[cfg(feature = "gui")]
mod gui;

// Safe settings files (atomic, .bak, a damaged file put back instead of starting over).
#[path = "../../hearth-apptheme/appfile.rs"]
mod appfile;

use std::path::PathBuf;

pub fn default_out() -> PathBuf { dirs::home_dir().unwrap_or_else(std::env::temp_dir).join("Applications") }

const HELP: &str = "AppImage Maker: turns .deb, .rpm, .tar.* and .zip apps into AppImages.

Usage:
  hearth-appimage                       open the window
  hearth-appimage FILE...               open the window and convert these
  hearth-appimage convert [options] FILE...

Options for convert:
  -o, --out DIR     where the AppImages go (default ~/Applications)
  --menu            also put each one in the app menu
  --no-fetch        don't download libraries this computer lacks from Fedora
  --no-libs         don't put libraries inside at all (only the package's own files)
  --json            print each report as JSON
  --version";

fn cli(args: &[String]) -> i32 {
    let mut opt = convert::Options { out_dir: default_out(), fetch: true, bundle: true, menu: false };
    let mut files = Vec::new();
    let mut json = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() { Some(d) => opt.out_dir = PathBuf::from(d), None => { eprintln!("{a} needs a folder"); return 2; } },
            "--menu" => opt.menu = true,
            "--no-fetch" => opt.fetch = false,
            "--no-libs" => opt.bundle = false,
            "--json" => json = true,
            "-h" | "--help" => { println!("{HELP}"); return 0; }
            s if s.starts_with('-') => { eprintln!("unknown option {s}\n\n{HELP}"); return 2; }
            f => files.push(PathBuf::from(f)),
        }
    }
    if files.is_empty() { eprintln!("no packages given\n\n{HELP}"); return 2; }
    let mut failed = 0;
    for f in files {
        eprintln!("== {}", f.display());
        let mut on = |e: convert::Event| match e {
            convert::Event::Step(s) => eprintln!("-- {s}"),
            convert::Event::Log(s) => eprintln!("   {s}"),
        };
        match convert::convert(&f, &opt, &mut on) {
            Ok(r) => {
                for w in &r.warnings { eprintln!("warning: {w}"); }
                if json { println!("{}", serde_json::to_string(&r).unwrap_or_default()); } else { println!("{}", r.output); }
            }
            Err(e) => { eprintln!("error: {}: {e}", f.display()); failed += 1; }
        }
    }
    if failed > 0 { 1 } else { 0 }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") { println!("hearth-appimage {}", env!("CARGO_PKG_VERSION")); return; }
    if args.first().map(|a| a == "convert").unwrap_or(false) { std::process::exit(cli(&args[1..])); }
    if args.iter().any(|a| a == "-h" || a == "--help") { println!("{HELP}"); return; }
    #[cfg(feature = "gui")]
    {
        let files: Vec<String> = args.iter().filter(|a| !a.starts_with('-')).cloned().collect();
        let full = args.iter().any(|a| a == "--fullscreen");
        gui::run(files, full);
    }
    #[cfg(not(feature = "gui"))]
    {
        if args.is_empty() { println!("{HELP}"); return; }
        std::process::exit(cli(&args));
    }
}
