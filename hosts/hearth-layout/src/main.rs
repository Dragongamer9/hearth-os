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
//!   hearth-layout now --json         today's desktop as a layout (the editor starts from it when none is in place)
//!   hearth-layout apply-json JSON    put this layout in place (the editor's changes, as you make them)
//!   hearth-layout check-json JSON    {"ok": bool, "problems": [...], "coming": [...]}
//!   hearth-layout describe PART TEXT [SKETCH]   Describe it: your AI's suggestion, checked (JSON; nothing is put in place)
//!   hearth-layout keep-feature JSON  keep a new feature (Keep this)
//!   hearth-layout features           your features (JSON) · feature-on ID true|false
//!   hearth-layout describe-app APP TEXT [SKETCH]   Describe it in a Hearth app (files, …): its layout and features, checked
//!   hearth-layout app-get APP · app-keep APP JSON · app-undo APP · app-reset APP   what an app keeps (Keep this, Undo, Back to the original)
//!   hearth-layout send-idea JSON     send an idea to Hearth's makers (only on Send; exactly the idea shown)
//!   hearth-layout describe-screen TEXT SCREEN_JSON [SKETCH]   a whole screen by your AI (with any blocks it made), checked
//!   hearth-layout check-screen JSON · ready-screens   is this screen usable · Mantel's ready-made desktops
//!   hearth-layout make-block TEXT · keep-block JSON · run-block ID [--once] · block-allow ID always|no|ask · blocks-mine · block-on ID true|false
//!                                    the blocks your AI makes (made.rs): made, kept, one boxed-in look, what you said to its ask
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
        "now" => {
            println!("{}", serde_json::to_string(&apply::today("mine", "Your desktop")).unwrap());
            0
        }
        "check-json" => {
            let (ok, problems, coming) = match layout::parse(&arg(1)) {
                Ok(l) => (true, Vec::new(), l.not_built()),
                Err(e) => (false, e, Vec::new()),
            };
            println!("{}", serde_json::json!({ "ok": ok, "problems": problems, "coming": coming }));
            if ok { 0 } else { 1 }
        }
        "apply-json" => match layout::parse(&arg(1)).map_err(|e| e.join(" ")).and_then(|l| apply::apply(&l).map(|_| l)) {
            Ok(l) => {
                window_buttons();
                println!("{} is in place.", l.name);
                0
            }
            Err(e) => fail(&e),
        },
        "describe" => {
            let sketch = args.get(3).filter(|s| !s.is_empty()).map(std::path::PathBuf::from);
            let current = if arg(1) == "bar" || arg(1) == "panel" || arg(1) == "desktop" {
                serde_json::to_value(apply::today("mine", "Your desktop")).unwrap_or_default()
            } else {
                serde_json::Value::Null
            };
            let r = hearth_layout::describe::describe(&arg(1), &arg(2), sketch.as_deref(), &current);
            println!("{r}");
            if r["ok"] == true { 0 } else { 1 }
        }
        "describe-app" | "app-get" | "app-keep" | "app-undo" | "app-reset" => {
            use hearth_layout::app;
            let Some(a) = app::find(&arg(1)) else { std::process::exit(fail(&format!("\"{}\" has no Describe it yet.", arg(1)))) };
            let r = match arg(0).as_str() {
                "describe-app" => {
                    let sketch = args.get(3).filter(|s| !s.is_empty()).map(std::path::PathBuf::from);
                    Ok(app::describe(a, &arg(2), sketch.as_deref()))
                }
                "app-get" => Ok(app::get(a.app)),
                "app-keep" => serde_json::from_str::<serde_json::Value>(&arg(2)).map_err(|e| e.to_string()).and_then(|v| app::keep(a, &v)),
                "app-undo" => app::undo(a.app),
                _ => app::reset(a.app),
            };
            match r {
                Ok(v) => {
                    println!("{v}");
                    if v.get("ok") == Some(&serde_json::json!(false)) { 1 } else { 0 }
                }
                Err(e) => fail(&e),
            }
        }
        // Build anything (screen.rs, made.rs): whole screens and the blocks your AI makes.
        "describe-screen" => {
            let sketch = args.get(3).filter(|s| !s.is_empty()).map(std::path::PathBuf::from);
            let current: serde_json::Value = serde_json::from_str(&arg(2)).unwrap_or(serde_json::Value::Null);
            let r = hearth_layout::describe::describe_screen(&arg(1), sketch.as_deref(), &current);
            println!("{r}");
            if r["ok"] == true { 0 } else { 1 }
        }
        "check-screen" => match serde_json::from_str::<serde_json::Value>(&arg(1)).map_err(|e| vec![e.to_string()]).and_then(|v| hearth_layout::screen::parse(&v)) {
            Ok(_) => { println!("{}", serde_json::json!({ "ok": true, "problems": [] })); 0 }
            Err(e) => { println!("{}", serde_json::json!({ "ok": false, "problems": e })); 1 }
        },
        "ready-screens" => {
            let v: Vec<serde_json::Value> = hearth_layout::screen::READY.iter().map(|(id, t)| serde_json::json!({ "id": id, "screen": serde_json::from_str::<serde_json::Value>(t).unwrap_or_default() })).collect();
            println!("{}", serde_json::Value::from(v));
            0
        }
        "make-block" => {
            let r = hearth_layout::made::make(&arg(1));
            println!("{r}");
            if r["ok"] == true { 0 } else { 1 }
        }
        "keep-block" => match serde_json::from_str::<serde_json::Value>(&arg(1)).map_err(|e| vec![e.to_string()]).and_then(|v| hearth_layout::made::parse(&v)).map_err(|e| e.join(" ")).and_then(|m| hearth_layout::made::keep(&m)) {
            Ok(()) => 0,
            Err(e) => fail(&e),
        },
        "run-block" => {
            // its own small program: boxed in (made.rs box_in), and ended after 10 s whatever it's doing
            std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(10)); println!("{}", serde_json::json!({ "ok": false, "error": "It took too long, so it was stopped." })); std::process::exit(3); });
            let r = hearth_layout::made::run(&arg(1), arg(2) == "--once");
            println!("{r}");
            0
        }
        "block-allow" => match hearth_layout::made::allow(&arg(1), match arg(2).as_str() { "always" => "always", "no" => "no", _ => "" }) {
            Ok(()) => 0,
            Err(e) => fail(&e),
        },
        "blocks-mine" => { println!("{}", hearth_layout::made::mine()); 0 }
        "block-on" => match hearth_layout::made::set_on(&arg(1), arg(2) == "true") {
            Ok(()) => 0,
            Err(e) => fail(&e),
        },
        "keep-feature" => match feature::parse(&arg(1)).and_then(|f| feature::keep(&f)) {
            Ok(p) => {
                println!("Kept: {}", p.display());
                0
            }
            Err(e) => fail(&e),
        },
        "features" => {
            let v: Vec<serde_json::Value> = feature::yours().into_iter().map(|(f, on)| serde_json::json!({ "feature": f, "on": on, "described": feature::describe(&f) })).collect();
            println!("{}", serde_json::to_string(&v).unwrap());
            0
        }
        "feature-on" => match feature::set_on(&arg(1), arg(2) == "true") {
            Ok(()) => 0,
            Err(e) => fail(&e),
        },
        "send-idea" => match serde_json::from_str::<hearth_layout::idea::Idea>(&arg(1)) {
            // cleaned again here: whatever comes in, only what idea.rs keeps goes out
            Ok(i) => {
                let i = hearth_layout::idea::Idea { kept: i.kept, ..hearth_layout::idea::Idea::new(&i.part, &i.asked, i.needs.clone(), i.closest.clone(), i.sketch, &i.hearth, &i.app) };
                let url = std::env::var("HEARTH_IDEAS_URL").unwrap_or_else(|_| "https://hearth-counter.game305-cl.workers.dev/v1/idea".into());
                let out = std::process::Command::new("curl")
                    .args(["-sS", "-m", "30", "-H", "Content-Type: application/json", "--data-binary", &i.to_json(), &url])
                    .output();
                match out {
                    Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).contains("\"key\"") => {
                        println!("{}", String::from_utf8_lossy(&o.stdout).trim());
                        0
                    }
                    Ok(o) => fail(&format!("It couldn't be sent ({}{}). Nothing else was sent.", String::from_utf8_lossy(&o.stdout).trim(), String::from_utf8_lossy(&o.stderr).trim())),
                    Err(e) => fail(&format!("It couldn't be sent ({e}).")),
                }
            }
            Err(e) => fail(&format!("That isn't an idea ({e}).")),
        },
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
