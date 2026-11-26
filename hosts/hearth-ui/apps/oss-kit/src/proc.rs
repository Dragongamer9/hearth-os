//! A program run with its input on stdin, off the window's thread. Passwords and tokens go this way (keepassxc-cli,
//! secret-tool), never on the command line, where `ps` would show them to any program on the PC.

use hearth_ui::run::Output;
use std::future::Future;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn blocking(cmd: &[String], input: &[u8], env: &[(String, String)]) -> Output {
    if cmd.is_empty() {
        return Output { ok: false, code: -1, out: String::new(), err: "nothing to run".into() };
    }
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in env {
        c.env(k, v);
    }
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => return Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input);
    }
    match child.wait_with_output() {
        Ok(o) => Output {
            ok: o.status.success(),
            code: o.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&o.stdout).to_string(),
            err: String::from_utf8_lossy(&o.stderr).to_string(),
        },
        Err(e) => Output { ok: false, code: -1, out: String::new(), err: format!("{}: {e}", cmd[0]) },
    }
}

pub fn spawn(cmd: Vec<String>, input: Vec<u8>) -> impl Future<Output = Output> + Send + 'static {
    spawn_env(cmd, input, Vec::new())
}

pub fn spawn_env(cmd: Vec<String>, input: Vec<u8>, env: Vec<(String, String)>) -> impl Future<Output = Output> + Send + 'static {
    let (tx, rx) = futures::channel::oneshot::channel();
    let _ = std::thread::Builder::new().name("hearth-oss-run".into()).spawn(move || {
        let mut input = input;
        let o = blocking(&cmd, &input, &env);
        // the input (a password) doesn't stay in memory longer than it's needed
        for b in input.iter_mut() {
            *b = 0;
        }
        let _ = tx.send(o);
    });
    async move { rx.await.unwrap_or(Output { ok: false, code: -1, out: String::new(), err: "it stopped".into() }) }
}
