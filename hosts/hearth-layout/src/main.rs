//! hearth-layout: desktop layouts from the command line (Settings › Layouts uses the same library).
//!   hearth-layout list               every layout (Hearth's and yours) and which is in place
//!   hearth-layout show ID|FILE       a layout's file
//!   hearth-layout check FILE         is this layout (or feature) usable? why not, in plain words
//!   hearth-layout apply ID|FILE      put it in place (what's there now is kept for Undo)
//!   hearth-layout undo               put back what was there (again: redo)
//!   hearth-layout save NAME          today's desktop as one of your layouts
//!   hearth-layout current            the layout in place

use hearth_layout::{apply, feature, layout};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let code = match arg(0).as_str() {
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
                println!("{} is in place. Undo: hearth-layout undo", l.name);
                0
            }
            Err(e) => fail(&e),
        },
        "undo" => match apply::undo() {
            Ok(()) => {
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

fn fail(e: &str) -> i32 {
    eprintln!("{e}");
    1
}
