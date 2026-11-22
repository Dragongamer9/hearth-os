//! `hearth-mantel session`: Desktop Mode's shell when the Rust Mantel test is installed (labwc's autostart runs it in
//! place of Quickshell; docs/MANTEL-RUST.md). It starts the wallpaper, the bar and the dock, each its own process, and
//! starts one again when it closes. If the bar can't stay up (it closes within 10 s three times in a row), everything
//! stops and this exits with 1: the autostart then marks the Store's copy as failed (hearth-part) and starts the QML
//! Mantel, so Desktop Mode always comes up (red flag 2).

use std::process::{Child, Command};
use std::time::{Duration, Instant};

const PARTS: [&str; 3] = ["wallpaper", "bar", "dock"];

fn start(me: &std::path::Path, part: &str) -> Option<Child> {
    match Command::new(me).arg(part).spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("hearth-mantel session: couldn't start the {part}: {e}");
            None
        }
    }
}

pub fn run() -> i32 {
    let Ok(me) = std::env::current_exe() else { return 1 };
    let mut kids: Vec<(String, Option<Child>, Instant, u32)> = PARTS.iter().map(|p| (p.to_string(), start(&me, p), Instant::now(), 0)).collect();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let mut give_up = false;
        for k in kids.iter_mut() {
            if k.3 == u32::MAX {
                continue; // left off
            }
            let gone = match &mut k.1 {
                Some(c) => matches!(c.try_wait(), Ok(Some(_)) | Err(_)),
                None => true,
            };
            if !gone {
                continue;
            }
            let quick = k.2.elapsed() < Duration::from_secs(10);
            k.3 = if quick { k.3 + 1 } else { 0 };
            if k.3 >= 3 {
                if k.0 == "bar" {
                    eprintln!("hearth-mantel session: the bar keeps closing right after it starts; back to the QML Mantel");
                    give_up = true;
                    break;
                }
                // The wallpaper or the dock: left off (the desktop still works), said once.
                eprintln!("hearth-mantel session: the {} keeps closing; left off until the next sign-in", k.0);
                k.1 = None;
                k.3 = u32::MAX;
                continue;
            }
            eprintln!("hearth-mantel session: the {} closed; starting it again", k.0);
            k.1 = start(&me, &k.0);
            k.2 = Instant::now();
        }
        if give_up {
            for o in kids.iter_mut() {
                if let Some(c) = &mut o.1 {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            }
            return 1;
        }
    }
}
