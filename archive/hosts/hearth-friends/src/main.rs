//! hearth-friends: Hearth Friends' service and its command line.
//!
//!   hearth-friends serve            run the service (Hearth's session starts it: hearth-friends.service; Hearth's
//!                                   core starts it too when it isn't running). One per user.
//!   hearth-friends call METHOD [JSON]   one request; prints the answer as JSON, or "error: ..." and exits 1
//!   hearth-friends watch [CLIENT]   prints events as JSON lines until stopped (Mantel's Friends panel)
//!
//! It speaks the protocol in src/proto.rs on $XDG_RUNTIME_DIR/hearth-friends.sock.

mod notify;
mod secrets;
mod service;

use hearth_friends::{proto, Paths};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("serve") {
        "serve" => serve(),
        "call" => {
            let Some(method) = args.get(2) else { eprintln!("usage: hearth-friends call METHOD [JSON]"); std::process::exit(2) };
            let a: Value = args.get(3).map(|s| serde_json::from_str(s).unwrap_or(json!({}))).unwrap_or(json!({}));
            let timeout = std::time::Duration::from_secs(if ["login", "register", "authStart", "recoveryEnable"].contains(&method.as_str()) { 90 } else { 45 });
            ensure_running();
            match proto::call(&hearth_friends::socket_path(), method, &a, timeout) {
                Ok(v) => println!("{v}"),
                Err(e) => { println!("error: {e}"); std::process::exit(1) }
            }
        }
        "watch" => watch(args.get(2).cloned().unwrap_or_else(|| "cli".into())),
        "--version" | "version" => println!("hearth-friends {}", env!("CARGO_PKG_VERSION")),
        _ => { eprintln!("usage: hearth-friends serve | call METHOD [JSON] | watch [CLIENT]"); std::process::exit(2) }
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio")
}

/// `call` and `watch` start the service when it isn't running (Mantel uses them; Hearth's session also starts it).
fn ensure_running() {
    let sock = hearth_friends::socket_path();
    if std::os::unix::net::UnixStream::connect(&sock).is_ok() { return; }
    let Ok(exe) = std::env::current_exe() else { return };
    let paths = Paths::from_env();
    let _ = std::fs::create_dir_all(&paths.data);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(paths.data.join("service.log")).ok();
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve").stdin(std::process::Stdio::null());
    match log.and_then(|l| l.try_clone().ok().map(|c| (l, c))) {
        Some((a, b)) => { cmd.stdout(a).stderr(b); }
        None => { cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()); }
    }
    if let Ok(mut child) = cmd.spawn() {
        std::thread::spawn(move || { let _ = child.wait(); });
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if std::os::unix::net::UnixStream::connect(&sock).is_ok() { return; }
        }
    }
}

fn watch(client: String) {
    use std::io::{BufRead, Write};
    ensure_running();
    let sock = hearth_friends::socket_path();
    let Ok(mut s) = std::os::unix::net::UnixStream::connect(&sock) else { println!("error: Hearth Friends isn't running"); std::process::exit(1) };
    let _ = s.write_all(proto::request(1, "watch", &json!({"client": client})).as_bytes());
    let out = std::io::stdout();
    for line in std::io::BufReader::new(s).lines().map_while(Result::ok) {
        let mut o = out.lock();
        if writeln!(o, "{line}").is_err() || o.flush().is_err() { break; }
    }
}

fn my_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

fn serve() {
    let paths = Paths::from_env();
    let sock = paths.socket.clone();
    // One service per user: a running one answers on the socket.
    if std::os::unix::net::UnixStream::connect(&sock).is_ok() { eprintln!("hearth-friends: already running"); return; }
    let _ = std::fs::remove_file(&sock);
    let rt = runtime();
    rt.block_on(async move {
        let listener = match UnixListener::bind(&sock) { Ok(l) => l, Err(e) => { eprintln!("hearth-friends: can't listen on {}: {e}", sock.display()); std::process::exit(1) } };
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600));
        }
        let svc = service::Service::new(paths);
        svc.restore().await;
        tokio::spawn(svc.clone().sync_loop());
        let me = my_uid();
        eprintln!("hearth-friends: listening");
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            // Only this user's programs (the socket is in their runtime folder and 0600 too).
            let peer = stream.peer_cred().ok().map(|c| c.uid());
            if me.is_some() && peer != me { continue; }
            tokio::spawn(client(svc.clone(), stream));
        }
    });
}

async fn client(svc: Arc<service::Service>, stream: UnixStream) {
    let (r, mut w) = stream.into_split();
    let mut lines = BufReader::new(r).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.len() > 1 << 20 { break; }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { let _ = w.write_all(proto::reply(&Value::Null, Err("Not JSON".into())).as_bytes()).await; continue };
        let id = msg["id"].clone();
        let method = msg["method"].as_str().unwrap_or("").to_string();
        let args = if msg["args"].is_object() { msg["args"].clone() } else { json!({}) };
        if method == "watch" {
            // From here on this connection only gets events.
            let gm = args["client"] == "gamemode";
            if gm { svc.gamemode_watchers.fetch_add(1, Ordering::SeqCst); }
            let mut rx = svc.events.subscribe();
            let _ = w.write_all(proto::reply(&id, Ok(svc.status_value().await)).as_bytes()).await;
            loop {
                tokio::select! {
                    ev = rx.recv() => match ev {
                        Ok((name, data)) => { if w.write_all(proto::event(&name, &data).as_bytes()).await.is_err() { break; } }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => { if w.write_all(proto::event("resync", &json!({})).as_bytes()).await.is_err() { break; } }
                        Err(_) => break,
                    },
                    // The watcher went away (closed its end): stop at once, so Game Mode's count is right.
                    l = lines.next_line() => { if !matches!(l, Ok(Some(_))) { break; } }
                }
            }
            if gm { svc.gamemode_watchers.fetch_sub(1, Ordering::SeqCst); }
            return;
        }
        // One request at a time per connection (Hearth's core opens one per request).
        let r = svc.handle(&method, &args).await;
        if w.write_all(proto::reply(&id, r).as_bytes()).await.is_err() { break; }
    }
}
