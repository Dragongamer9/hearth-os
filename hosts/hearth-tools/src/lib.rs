//! Hearth Tools: Hearth OS's everyday system tools, seven apps in one program (`hearth-tools --app <name>`):
//! Disk Usage, System Info, Logs, Network Tools, Passwords & Keys, Printers & Scanner, Welcome & Tips.
//! One program because they share everything but their page: the window (Tauri: WebKitGTK), the look and
//! controller handling (ui/common.*), and the helpers here; one build instead of seven. Each app has its own
//! menu entry, icon and window (its own .desktop file runs it with --app).
//!
//! The page calls `call(method, args)` for quick answers and `start(job, args)` for long work that reports as it
//! goes (scans, the speed test, ping...). Both land in `call` / `job` below.

pub mod diskusage;
pub mod logs;
pub mod network;
pub mod printers;
pub mod secrets;
pub mod sysinfo;
pub mod util;
pub mod welcome;

use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

/// The apps: (--app name, window title, page).
pub const APPS: [(&str, &str, &str); 7] = [
    ("disk-usage", "Disk Usage", "disk-usage.html"),
    ("system-info", "System Info", "system-info.html"),
    ("logs", "Logs", "logs.html"),
    ("network", "Network Tools", "network.html"),
    ("passwords", "Passwords & Keys", "passwords.html"),
    ("printers", "Printers & Scanner", "printers.html"),
    ("welcome", "Welcome & Tips", "welcome.html"),
];

pub fn app(name: &str) -> Option<(&'static str, &'static str, &'static str)> { APPS.iter().copied().find(|a| a.0 == name) }

fn strings(args: &Value, key: &str) -> Vec<String> {
    args.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// Quick calls from the pages.
pub fn call(method: &str, args: &Value) -> Result<Value, String> {
    match method {
        "du.places" => diskusage::places(),
        "du.view" => diskusage::view(util::arg(args, "path"), util::arg_u64(args, "depth", 3) as u32),
        "du.list" => diskusage::list(util::arg(args, "path")),
        "du.bigFiles" => diskusage::big_files(),
        "du.trash" => diskusage::trash(&strings(args, "paths")),
        "du.emptyTrash" => diskusage::empty_trash(),
        "du.open" => diskusage::open(util::arg(args, "path")),

        "sys.overview" => sysinfo::overview(),

        "logs.boots" => logs::boots(),
        "logs.apps" => logs::apps(args),
        "logs.query" => logs::query(args),
        "logs.files" => logs::files(),
        "logs.file" => logs::file(args),
        "logs.export" => logs::export(args),
        "logs.openFolder" => diskusage::open(&util::home().to_string_lossy()),

        "net.overview" => network::overview(),
        "net.wifi" => network::wifi(args),
        "net.connections" => network::connections(),
        "net.usage" => network::usage(),
        "net.dns" => network::dns(args),

        "keys.list" => secrets::keyring_list(),
        "keys.reveal" => secrets::keyring_reveal(args),
        "keys.delete" => secrets::keyring_delete(args),
        "keys.wifi" => secrets::wifi_list(),
        "keys.wifiReveal" => secrets::wifi_reveal(args),
        "keys.wifiForget" => secrets::wifi_forget(args),

        "print.state" => printers::state(),
        "print.job" => printers::job_action(args),
        "print.printer" => printers::printer_action(args),
        "print.discover" => printers::discover(),
        "print.add" => printers::add(args),
        "print.settings" => printers::open_settings(),
        "scan.devices" => printers::devices(),
        "scan.options" => printers::options(args),
        "scan.clear" => printers::pages_clear(args),
        "scan.save" => printers::save(args),
        "scan.openFolder" => diskusage::open(&util::user_dir("DOCUMENTS", "Documents").join("Scans").to_string_lossy()),

        "app.info" => welcome::info(),
        "welcome.done" => welcome::done(),
        "app.open" => welcome::open(args),
        _ => Err(format!("unknown call {method}")),
    }
}

/// Long jobs: `emit` gets each step ({...} progress or output lines); the return value is the result.
pub fn job(name: &str, args: &Value, cancel: &AtomicBool, emit: &(dyn Fn(Value) + Sync)) -> Result<Value, String> {
    match name {
        "du.scan" => diskusage::scan(util::arg(args, "path"), cancel, emit),
        "du.quick" => diskusage::quick(util::arg(args, "kind"), cancel),
        "net.speed" => network::speed(cancel, emit),
        "net.ping" => network::ping(args, cancel, emit),
        "net.trace" => network::trace(args, cancel, emit),
        "logs.send" => logs::send(args),
        "keys.list" => secrets::keyring_list(),
        "keys.reveal" => secrets::keyring_reveal(args),
        "scan.preview" => printers::scan(args, true, cancel, emit),
        "scan.page" => printers::scan(args, false, cancel, emit),
        "scan.devices" => printers::devices(),
        "print.discover" => printers::discover(),
        "sys.overview" => sysinfo::overview(),
        _ => Err(format!("unknown job {name}")),
    }
}

/// For checks without a window: `hearth-tools --call METHOD [JSON]`.
pub fn cli_call(method: &str, args: &str) -> i32 {
    let a: Value = serde_json::from_str(if args.is_empty() { "{}" } else { args }).unwrap_or(json!({}));
    let stop = AtomicBool::new(false);
    let r = match call(method, &a) {
        Err(e) if e.starts_with("unknown call") => job(method, &a, &stop, &|v| eprintln!("{v}")),
        other => other,
    };
    match r {
        Ok(v) => { println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}
