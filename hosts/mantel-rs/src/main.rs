//! hearth-mantel: Mantel (Hearth Classic) in Rust, a test (docs/MANTEL-RUST.md). Opt in only, through the Hearth
//! Store's test "Mantel in Rust"; the QML Mantel stays the default, and the one that starts whenever this one can't.
//!
//!   hearth-mantel session      Desktop Mode's shell, one program: the wallpaper, the bar, the dock and the app menu
//!                              (labwc's autostart runs it in place of Quickshell when the Store's copy is installed)
//!   hearth-mantel menu         opens or closes the app menu of the running shell (labwc's Super key)
//!   hearth-mantel check        what it would show, as text (tests): the look, the apps, the pins, the status

mod apps;
mod bar;
mod dock;
mod look;
mod menu;
mod shell;
mod sys;
mod wall;
mod wm;

const USAGE: &str = "usage: hearth-mantel session | menu | check";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("session") => shell::run(),
        Some("menu") => {
            if sys::send_command("menu") { 0 } else { 1 }
        }
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
    let l = look::get();
    println!("look {} style={} dark={} accent={:?} glass={} font={:?}", l.name, l.style, l.p.dark, l.p.accent, l.glass_a, l.font);
    println!("wallpaper {:?}", wall::scene(&l));
    println!("bar band {} · dock {} (keeps {})", bar::band(&l, false), dock::height(&l, false), dock::reserved(&l, false));
    println!("icons {}", ["bell", "kbd", "wifi", "pad", "search", "gear"].iter().filter(|n| look::icon_handle(n, l.p.text).is_some()).count());
    let a = apps::all();
    println!("apps {}", a.len());
    for x in a.iter().take(12) {
        println!("  {} | {} | icon={}", x.id, x.name, x.icon_path.as_deref().map(|p| p.display().to_string()).unwrap_or_default());
    }
    println!("pins {:?}", sys::pins());
    let s = sys::Status::read(true);
    println!("status clock={} wifi={:?} battery={:?} volume={:?} bt={:?}", sys::clock_text(), s.wifi, s.battery, s.volume, sys::Bt::read());
    0
}

/// Why the shell couldn't start (its log).
pub fn ended(name: &str, r: Result<(), iced_layershell::Error>) -> i32 {
    match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("hearth-mantel {name}: {e}");
            1
        }
    }
}
