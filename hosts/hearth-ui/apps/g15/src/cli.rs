//! hearth-g15: Hearth's Logitech G15 service and its command line (docs/G15.md).
//!
//!   hearth-g15 service              the user service (hearth-g15.service): G-keys, macros, the screen, the lights
//!   hearth-g15 present              exit 0 when a G15 is plugged in (the unit's ExecCondition), 1 when not
//!   hearth-g15 status               one JSON line: the keyboard, what it can do here, its lights
//!   hearth-g15 light kbd|lcd 0|1|2  set the key light or the screen's light, and remember it (g15.json)
//!   hearth-g15 invert on|off        light dots on dark on the keyboard's screen (Keyboard screen › Dots), remembered
//!   hearth-g15 frame PAGE OUT.png [--scale N] [--report OUT.bin]
//!                                   a picture of a screen page (clock, system, music, alerts, rec, set, alert or your
//!                                   own screen's id), as the keyboard would show it
//!   hearth-g15 make-fixture DIR     a pretend G15 for CI (then HEARTH_G15_FIXTURE=DIR)

use hearth_g15::{config, device, leds, lcd, root::Root, service};
use serde_json::json;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let root = Root::from_env();
    let code = match a.as_slice() {
        ["service"] => match device::find(&root) {
            Some(d) => service::Service::new(root, d).run(),
            None => {
                hearth_g15::log("no G15 plugged in");
                0
            }
        },
        ["present"] => {
            if device::find(&root).is_some() {
                0
            } else {
                1
            }
        }
        ["status"] => {
            println!("{}", service::status(&root));
            0
        }
        ["light", which @ ("kbd" | "lcd"), v] => {
            let Ok(v) = v.parse::<u32>() else {
                println!("error: the level is 0, 1 or 2");
                std::process::exit(1);
            };
            let v = v.min(2);
            let Some(d) = device::find(&root) else {
                println!("error: no G15 is plugged in");
                std::process::exit(1);
            };
            let dir = if *which == "kbd" { d.leds.kbd } else { d.leds.lcd };
            let key = if *which == "kbd" { "kbdLight" } else { "lcdLight" };
            match dir.map(|p| leds::set(&p, v)) {
                Some(Ok(())) => match config::update(|m| {
                    m.insert(key.into(), json!(v));
                }) {
                    Ok(()) => {
                        println!("{}", json!({"ok": true, "light": which, "level": v}));
                        0
                    }
                    Err(e) => {
                        println!("error: couldn't remember it ({e})");
                        1
                    }
                },
                Some(Err(e)) => {
                    println!("error: couldn't change the light ({e})");
                    1
                }
                None => {
                    println!("error: this keyboard has no such light");
                    1
                }
            }
        }
        ["invert", v @ ("on" | "off")] => {
            let on = *v == "on";
            match config::update(|m| {
                m.insert("lcdInvert".into(), json!(on));
            }) {
                Ok(()) => {
                    println!("{}", json!({"ok": true, "invert": on}));
                    0
                }
                Err(e) => {
                    println!("error: couldn't remember it ({e})");
                    1
                }
            }
        }
        ["frame", page, out, rest @ ..] => {
            let mut scale = 5u32;
            let mut report: Option<&str> = None;
            let mut i = 0;
            while i < rest.len() {
                match rest[i] {
                    "--scale" if i + 1 < rest.len() => {
                        scale = rest[i + 1].parse().unwrap_or(5);
                        i += 1;
                    }
                    "--report" if i + 1 < rest.len() => {
                        report = Some(rest[i + 1]);
                        i += 1;
                    }
                    _ => {}
                }
                i += 1;
            }
            match service::frame_of(root, page) {
                Some(f) => {
                    let ok = std::fs::write(out, lcd::frame_png(&f, scale)).is_ok() && report.map(|r| std::fs::write(r, lcd::pack(&f)).is_ok()).unwrap_or(true);
                    if ok {
                        0
                    } else {
                        println!("error: couldn't write {out}");
                        1
                    }
                }
                None => {
                    println!("error: no G15 here, or no page called {page}");
                    1
                }
            }
        }
        ["make-fixture", dir] => match service::make_fixture(Path::new(dir)) {
            Ok(()) => 0,
            Err(e) => {
                println!("error: {e}");
                1
            }
        },
        _ => {
            eprintln!("usage: hearth-g15 service | present | status | light kbd|lcd 0|1|2 | invert on|off | frame PAGE OUT.png [--scale N] [--report OUT.bin] | make-fixture DIR");
            2
        }
    };
    std::process::exit(code);
}
