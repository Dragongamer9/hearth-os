//! firehold-shell: Firehold's shell, all in Rust (docs/FIREHOLD-RUST.md; a prototype for the Anatase test).
//!
//!   firehold-shell hub                 the session's shell: starts the surfaces, talks to firehold-comp (hub.rs)
//!   firehold-shell couch|bar|dock|guide|notice|perf|settings
//!                                      one surface each (the hub starts them)
//!   firehold-shell ctl LINE            one command to the running hub (tests): `ctl state`, `ctl tab library`,
//!                                      `ctl posture desk`, `ctl page switcher`, `ctl guide`, `ctl pad a`
//!   firehold-shell chosen              exit 0 when Desktop Mode is "Firehold (Rust, test)" (nimbus-session asks)
//!   firehold-shell choose firehold|plasma
//!                                      the Desktop Mode choice, saved with settings_file.rs only when it changes

mod config;
mod hub;
mod link;
mod model;
mod proto;
mod sys;
mod ui;

/// Hearth's one safe way to read and write a settings file, shared with hearth-core (docs/SETTINGS-PERSISTENCE.md).
#[path = "../../hearth-core/src/settings_file.rs"]
#[allow(dead_code)]
mod settings_file;

/// What settings_file.rs needs from its crate.
mod paths {
    pub fn home() -> std::path::PathBuf {
        crate::config::home()
    }
}
#[cfg(test)]
mod admin {
    /// hearth-core's, for settings_file.rs's own test.
    pub fn write_start_mode(text: &str, mode: &str) -> String {
        let line = format!("NIMBUS_START_MODE={mode}");
        if text.lines().any(|l| l.trim_start().starts_with("NIMBUS_START_MODE=")) {
            let mut out: Vec<String> = text.lines().map(|l| if l.trim_start().starts_with("NIMBUS_START_MODE=") { line.clone() } else { l.to_string() }).collect();
            out.push(String::new());
            out.join("\n")
        } else {
            format!("{}{}{line}\n", text, if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" })
        }
    }
}

const USAGE: &str = "usage: firehold-shell hub | couch | bar | dock | guide | notice | perf | settings | ctl LINE | chosen | choose firehold|plasma";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("hub") => hub::run(),
        Some("couch") => ui::couch::run(),
        Some("bar") => ui::desk::run("bar"),
        Some("dock") => ui::desk::run("dock"),
        Some("notice") => ui::desk::run("notice"),
        Some("guide") => ui::guide::run(false),
        Some("perf") => ui::guide::run(true),
        Some("settings") => ui::settings::run(),
        Some("ctl") => hub::ctl(&args[1..]),
        Some("chosen") => {
            let c = config::desktop_choice();
            println!("{c}");
            if c == "firehold" {
                0
            } else {
                1
            }
        }
        Some("choose") => match config::choose(args.get(1).map(String::as_str).unwrap_or("")) {
            Ok(changed) => {
                println!("{}", if changed { "saved" } else { "unchanged" });
                0
            }
            Err(e) => {
                eprintln!("firehold-shell choose: {e}");
                2
            }
        },
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}
