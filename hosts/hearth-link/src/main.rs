//! hearth-link: the Hearth Link service and its command line.
//!
//!   hearth-link serve                   the service (the systemd user unit hearth-link.service runs this)
//!   hearth-link call <command> [JSON]   one request to the running service (what the UI sends), its answer as JSON
//!   hearth-link status                  = call status
//!   hearth-link id                      this device's id (its public key)

use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(String::as_str) {
        Some("serve") => serve(),
        Some("call") => {
            let cmd = args.get(2).cloned().unwrap_or_default();
            let a: Value = args.get(3).and_then(|s| serde_json::from_str(s).ok()).unwrap_or(serde_json::json!({}));
            let t = std::env::var("HEARTH_LINK_CALL_TIMEOUT").ok().and_then(|t| t.parse().ok()).unwrap_or(120);
            print_result(hearth_link::ipc::call(&cmd, a, t))
        }
        Some("status") => print_result(hearth_link::ipc::call("status", serde_json::json!({}), 20)),
        Some("id") => match hearth_link::state::load_or_make_key(&hearth_link::home_dir()) {
            Ok(k) => { println!("{}", k.public()); 0 }
            Err(e) => { eprintln!("{e}"); 1 }
        },
        _ => {
            eprintln!("usage: hearth-link serve | call <command> [JSON] | status | id");
            2
        }
    };
    std::process::exit(code);
}

fn print_result(r: Result<Value, String>) -> i32 {
    match r {
        Ok(v) => { println!("{}", serde_json::to_string(&v).unwrap_or_default()); 0 }
        Err(e) => { println!("{}", serde_json::json!({"error": e})); 1 }
    }
}

fn serve() -> i32 {
    let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() { Ok(r) => r, Err(e) => { eprintln!("{e}"); return 1; } };
    rt.block_on(async {
        let node = match hearth_link::node::Node::start().await { Ok(n) => n, Err(e) => { eprintln!("hearth-link: {e}"); return 1; } };
        node.spawn_all();
        let path = hearth_link::socket_path();
        let n2 = node.clone();
        tokio::spawn(async move { if let Err(e) = hearth_link::ipc::serve(n2, &path).await { eprintln!("hearth-link: {e}"); std::process::exit(1); } });
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signals");
        tokio::select! { _ = term.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
        node.shutdown().await;
        let _ = std::fs::remove_file(hearth_link::socket_path());
        0
    })
}
