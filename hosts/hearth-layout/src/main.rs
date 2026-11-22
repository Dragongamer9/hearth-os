//! hearth-layout: desktop layouts from the command line (Settings › Layouts uses the same library).
//!   hearth-layout list               every layout (Hearth's and yours) and which is in place
//!   hearth-layout show ID|FILE       a layout's file
//!   hearth-layout check FILE         is this layout (or feature) usable? why not, in plain words
//!   hearth-layout apply ID|FILE      put it in place (what's there now is kept for Undo)
//!   hearth-layout undo               put back what was there (again: redo)
//!   hearth-layout save NAME          today's desktop as one of your layouts
//!   hearth-layout current            the layout in place
//!   hearth-layout list --json        the same as JSON, for Settings' Desktop layout page (with each layout whole)
//!   hearth-layout blocks --json      every block, where it may go and whether both Mantels draw it yet
//! After apply and undo the window buttons are put on (hearth-system window-buttons: labwc or KWin).

use hearth_layout::{apply, feature, layout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let code = match arg(0).as_str() {
        "list" if arg(1) == "--json" => {
            let (all, _) = apply::all();
            let cur = apply::current();
            let v: Vec<serde_json::Value> = all
                .iter()
                .map(|(l, mine)| serde_json::json!({ "id": l.id, "name": l.name, "about": l.about, "mine": mine, "inUse": l.id == cur, "coming": l.not_built(), "layout": l }))
                .collect();
            println!("{}", serde_json::to_string(&v).unwrap());
            0
        }
        "blocks" => {
            let v: Vec<serde_json::Value> = hearth_layout::blocks::BLOCKS
                .iter()
                .map(|b| {
                    serde_json::json!({ "id": b.id, "name": b.name, "bar": hearth_layout::blocks::in_bar(b.id), "panel": hearth_layout::blocks::in_panel(b.id), "built": b.built, "keep": b.keep })
                })
                .collect();
            println!("{}", serde_json::to_string(&v).unwrap());
            0
        }
        "list" => {
            let (all, bad) = apply::all();
            let cur = apply::current();
            for (l, mine) in all {
                let coming = l.not_built();
                println!(
                    "{} {:<12} {}{}{}",
                    if l.id == cur { "*" } else { " " },
                    l.id,
                    l.name,
                    if mine { " (yours)" } else { "" },
                    if coming.is_empty() { String::new() } else { format!(" — coming: {}", coming.join(", ")) }
                );
            }
            for (f, why) in bad {
                eprintln!("left out {}: {why}", f.display());
            }
            0
        }
        "show" => match apply::find(&arg(1)) {
            Ok(l) => {
                println!("{}", serde_json::to_string_pretty(&l).unwrap());
                0
            }
            Err(e) => fail(&e),
        },
        "check" => {
            let t = std::fs::read_to_string(arg(1)).unwrap_or_default();
            let v: serde_json::Value = serde_json::from_str(&t).unwrap_or_default();
            if v.get("part").is_some() {
                match feature::parse(&t) {
                    Err(e) => fail(&e),
                    Ok(f) => {
                        let c = feature::check(&f);
                        println!("{}", feature::describe(&f));
                        c.problems.iter().for_each(|p| println!("problem: {p}"));
                        c.needs.iter().for_each(|n| println!("needs new work: {n}"));
                        if c.usable() { 0 } else { 1 }
                    }
                }
            } else {
                match layout::parse(&t) {
                    Ok(l) => {
                        println!("{} can be used.", l.name);
                        let c = l.not_built();
                        if !c.is_empty() {
                            println!("Coming (left out until built): {}", c.join(", "));
                        }
                        0
                    }
                    Err(e) => {
                        e.iter().for_each(|x| println!("problem: {x}"));
                        1
                    }
                }
            }
        }
        "apply" => match apply::find(&arg(1)).and_then(|l| apply::apply(&l).map(|_| l)) {
            Ok(l) => {
                window_buttons();
                println!("{} is in place. Undo: hearth-layout undo", l.name);
                0
            }
            Err(e) => fail(&e),
        },
        "undo" => match apply::undo() {
            Ok(()) => {
                window_buttons();
                println!("Put back.");
                0
            }
            Err(e) => fail(&e),
        },
        "save" => match apply::save(&args[1.min(args.len())..].join(" ")) {
            Ok(p) => {
                println!("Saved as {}", p.display());
                0
            }
            Err(e) => fail(&e),
        },
        "current" => {
            println!("{}", apply::current());
            0
        }
        _ => {
            eprintln!("hearth-layout list | show ID | check FILE | apply ID|FILE | undo | save NAME | current");
            2
        }
    };
    std::process::exit(code);
}

/// The window buttons the layout chose, put on now (labwc's folder made again and reloaded, or KWin's). Best effort:
/// the layout is saved either way, and labwc's folder is made with them at the next sign-in.
fn window_buttons() {
    let share = std::env::var("NIMBUS_SHARE").unwrap_or_else(|_| "/usr/share/nimbus".into());
    let tool = std::path::Path::new(&share).join("desktop/shell/hearth-system");
    if tool.exists() {
        let _ = std::process::Command::new("python3").arg(tool).arg("window-buttons").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
    }
}

fn fail(e: &str) -> i32 {
    eprintln!("{e}");
    1
}
