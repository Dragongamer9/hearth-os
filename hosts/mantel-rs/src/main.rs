//! hearth-mantel: Mantel (Hearth Classic) in Rust, a test (docs/MANTEL-RUST.md). Opt in only, through the Hearth
//! Store's test "Mantel in Rust"; the QML Mantel stays the default, and the one that starts whenever this one can't.
//!
//!   hearth-mantel session      Desktop Mode's shell: starts the wallpaper, the bar and the dock and keeps them up
//!                              (labwc's autostart runs it in place of Quickshell when the Store's copy is installed)
//!   hearth-mantel bar | dock | wallpaper | menu
//!                              one surface each (the session starts them; the bar's flame opens and closes the menu)
//!   hearth-mantel check        what it would show, as text (tests): the look, the apps, the pins, the status

mod apps;
mod bar;
mod dock;
mod look;
mod menu;
mod session;
mod sys;
mod wall;
mod wm;

const USAGE: &str = "usage: hearth-mantel session | bar | dock | wallpaper | menu | check";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("session") => session::run(),
        Some("bar") => bar::run(),
        Some("dock") => dock::run(),
        Some("wallpaper") => wall::run(),
        Some("menu") => menu::run(),
        Some("check") => check(),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}

/// What the surfaces would show, without a screen (CI's first look; tests).
fn check() -> i32 {
    let p = look::pal();
    println!("look {} dark={} accent={:?}", look::look_name(), p.dark, p.accent);
    println!("wallpaper {:?}", wall::picture());
    let a = apps::all();
    println!("apps {}", a.len());
    for x in a.iter().take(12) {
        println!("  {} | {} | icon={}", x.id, x.name, x.icon_path.as_deref().map(|p| p.display().to_string()).unwrap_or_default());
    }
    println!("pins {:?}", sys::pins());
    let s = sys::Status::read(true);
    println!("status clock={} wifi={:?} battery={:?} volume={:?}", sys::clock_text(), s.wifi, s.battery, s.volume);
    0
}

/// Starts a layer surface's iced program and says why it couldn't (the session's log).
pub fn ended(name: &str, r: Result<(), iced_layershell::Error>) -> i32 {
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("hearth-mantel {name}: {e}");
            1
        }
    }
}
