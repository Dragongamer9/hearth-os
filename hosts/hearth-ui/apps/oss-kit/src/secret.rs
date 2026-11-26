//! Tokens kept in the desktop's own keyring (the Secret Service: KDE Wallet on Plasma, GNOME Keyring elsewhere)
//! through `secret-tool` (libsecret), the secret on its stdin. Never in a file, never on a command line, never
//! logged. Without a keyring the app asks again next time instead of saving it anywhere.

use crate::proc;
use hearth_ui::run::Output;
use std::future::Future;

fn attrs(app: &str, key: &str) -> Vec<String> {
    vec!["hearth-app".into(), app.into(), "key".into(), key.into()]
}

pub fn available() -> bool {
    crate::which(&["secret-tool"], &[]).is_some()
}

pub fn store(app: &str, key: &str, label: &str, secret: String) -> impl Future<Output = Output> + Send + 'static {
    let mut c = vec!["secret-tool".to_string(), "store".into(), format!("--label={label}")];
    c.extend(attrs(app, key));
    proc::spawn(c, secret.into_bytes())
}

/// The token, or "" (none kept, or no keyring).
pub fn lookup(app: &str, key: &str) -> impl Future<Output = String> + Send + 'static {
    let mut c = vec!["secret-tool".to_string(), "lookup".into()];
    c.extend(attrs(app, key));
    let f = proc::spawn(c, Vec::new());
    async move {
        let o = f.await;
        if o.ok {
            o.out.trim_end_matches('\n').to_string()
        } else {
            String::new()
        }
    }
}

pub fn clear(app: &str, key: &str) -> impl Future<Output = Output> + Send + 'static {
    let mut c = vec!["secret-tool".to_string(), "clear".into()];
    c.extend(attrs(app, key));
    proc::spawn(c, Vec::new())
}
