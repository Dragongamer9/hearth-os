//! hearthfetch: Hearth OS's own summary of the device for the terminal, like fastfetch or neofetch. The command;
//! what it finds and how it's drawn are in the library (lib.rs).

use hearthfetch::{collect, enable_ansi, render, to_json, Colors};
use std::io::{IsTerminal, Write};

const HELP: &str = "hearthfetch: a short summary of this device and Hearth OS

Usage: hearthfetch [options]

  --json            print the summary as JSON (for scripts)
  --no-logo         leave out Hearth's mark
  --color <when>    auto (the default), always, 256 or never
  --time            say how long it took (on the error output)
  -h, --help        show this help
  -V, --version     show hearthfetch's version

Colors are left out when the output isn't a terminal or NO_COLOR is set.";

fn color_mode(flag: Option<&str>) -> Colors {
    match flag {
        Some("never") | Some("none") | Some("plain") => return Colors::Plain,
        Some("always") | Some("truecolor") | Some("24bit") => { enable_ansi(); return Colors::True; }
        Some("256") => { enable_ansi(); return Colors::X256; }
        _ => {}
    }
    if std::env::var_os("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false) { return Colors::Plain; }
    if !std::io::stdout().is_terminal() { return Colors::Plain; }
    if !enable_ansi() { return Colors::Plain; }
    let colorterm = std::env::var("COLORTERM").unwrap_or_default().to_lowercase();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") || cfg!(windows) { return Colors::True; }
    Colors::X256
}

fn main() {
    let start = std::time::Instant::now();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json_out = false;
    let mut with_logo = true;
    let mut time = false;
    let mut color: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => { println!("{HELP}"); return; }
            "-V" | "--version" => { println!("hearthfetch {}", env!("CARGO_PKG_VERSION")); return; }
            "--json" => json_out = true,
            "--no-logo" => with_logo = false,
            "--time" => time = true,
            "--color" | "--colour" => color = it.next().cloned(),
            s if s.starts_with("--color=") || s.starts_with("--colour=") => color = s.split_once('=').map(|x| x.1.to_string()),
            other => { eprintln!("hearthfetch: unknown option {other}\n\n{HELP}"); std::process::exit(2); }
        }
    }
    let info = collect();
    let text = if json_out {
        serde_json::to_string_pretty(&to_json(&info)).unwrap_or_default() + "\n"
    } else {
        render(&info, color_mode(color.as_deref()), with_logo)
    };
    let mut so = std::io::stdout().lock();
    let _ = so.write_all(text.as_bytes());
    let _ = so.flush();
    if time { eprintln!("hearthfetch took {} ms", start.elapsed().as_millis()); }
}
