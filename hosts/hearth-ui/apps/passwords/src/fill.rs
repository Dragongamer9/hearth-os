//! Filling passwords into browsers and other programs (board 16, Filling it in). Hearth's own way, no browser
//! extension needed:
//!   * while your vault is open, Hearth Passwords answers on a socket only you can reach
//!     ($XDG_RUNTIME_DIR/hearth/passwords.sock, 0600 in your own 0700 folder): which logins fit a window ("find"),
//!     and one login's name and password ("get"). Locking the vault (or closing the app) takes them away;
//!   * Super + Alt + P (labwc's keybind: `hearth-passwords-app --fill`) asks Mantel which window is in front
//!     (`hearth-mantel-ipc call pad state`), asks the vault for the logins that fit it (the website's name in a
//!     browser's title, or the item's name in an app's), and types the user name, Tab and the password into it through
//!     the Wayland virtual keyboard (wtype, the text on its stdin, never on a command line);
//!   * "Ask before filling" (on): a small window shows the logins that fit first, and only the one you pick is typed;
//!     "Also for apps (Hearth AI, Store)" (on): windows that aren't browsers too; "Fill in by itself in browsers"
//!     (off): with it on and the vault open, a browser window whose page fits a login gets it filled when it comes to
//!     the front (asked first, unless Ask is off), once a page.
//! $HEARTH_FILL_FRONT ("app|title"): the window in front (CI, where Mantel isn't running).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Mutex;
use zeroize::Zeroize;

#[derive(Clone, Default)]
pub struct Login {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub user: String,
    pub password: String,
}

impl Drop for Login {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

static SHARED: Mutex<Vec<Login>> = Mutex::new(Vec::new());
static SERVING: Mutex<bool> = Mutex::new(false);

pub const BROWSERS: [&str; 12] = ["firefox", "firefox-esr", "org.mozilla.firefox", "librewolf", "chromium", "chromium-browser", "google-chrome", "brave-browser", "vivaldi-stable", "microsoft-edge", "hearth-browser", "zen"];

pub fn sock() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("hearth/passwords.sock")
}

/// The word a website is known by ("https://github.com/login" → "github"; "accounts.google.com" → "google").
pub fn site_word(uri: &str) -> String {
    let host = uri.split("://").nth(1).unwrap_or(uri).split(['/', ':', '?']).next().unwrap_or("").to_lowercase();
    let parts: Vec<&str> = host.split('.').filter(|p| !p.is_empty()).collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        n => {
            // co.uk, com.au…: the word before them
            let second = parts[n - 2];
            if n >= 3 && matches!(second, "co" | "com" | "org" | "net" | "gov" | "ac") && parts[n - 1].len() == 2 { parts[n - 3].to_string() } else { second.to_string() }
        }
    }
}

/// The logins that fit a window's title, best first.
pub fn fits<'a>(title: &str, logins: &'a [Login]) -> Vec<&'a Login> {
    let t = title.to_lowercase();
    let mut v: Vec<(u32, &Login)> = logins
        .iter()
        .filter_map(|l| {
            let w = site_word(&l.uri);
            let n = l.name.to_lowercase();
            let s = if w.len() >= 3 && t.contains(&w) { 2 } else if n.len() >= 3 && t.contains(&n) { 1 } else { 0 };
            (s > 0 && !l.password.is_empty()).then_some((s, l))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.cmp(&b.1.name)));
    v.into_iter().map(|x| x.1).collect()
}

/// What the open vault offers (empty: locked). Starts answering on the socket the first time.
pub fn share(logins: Vec<Login>) {
    if let Ok(mut g) = SHARED.lock() {
        *g = logins;
    }
    let mut on = match SERVING.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if *on {
        return;
    }
    let p = sock();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700));
    }
    let _ = std::fs::remove_file(&p);
    let Ok(l) = UnixListener::bind(&p) else { return };
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    }
    *on = true;
    let _ = std::thread::Builder::new().name("hearth-passwords-fill".into()).stack_size(256 * 1024).spawn(move || {
        for c in l.incoming().flatten() {
            let _ = answer(c);
        }
    });
}

/// The app is closing: nothing more to answer.
pub fn stop() {
    if let Ok(mut g) = SHARED.lock() {
        g.clear();
    }
    let _ = std::fs::remove_file(sock());
}

fn answer(c: UnixStream) -> std::io::Result<()> {
    let _ = c.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut r = BufReader::new(c.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let q: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
    let g = SHARED.lock().map(|g| g.clone()).unwrap_or_default();
    let out = if g.is_empty() {
        json!({"locked": true})
    } else if let Some(title) = q.get("find").and_then(Value::as_str) {
        json!({"items": fits(title, &g).iter().map(|l| json!({"id": l.id, "name": l.name, "user": l.user, "site": site_word(&l.uri)})).collect::<Vec<_>>()})
    } else if let Some(id) = q.get("get").and_then(Value::as_str) {
        match g.iter().find(|l| l.id == id) {
            Some(l) => json!({"user": l.user, "password": l.password}),
            None => json!({"error": "no such login"}),
        }
    } else {
        json!({"error": "?"})
    };
    let mut w = c;
    w.write_all(format!("{out}\n").as_bytes())
}

/// One question to the open vault.
pub fn ask(q: &Value) -> Result<Value, String> {
    let mut c = UnixStream::connect(sock()).map_err(|_| "closed".to_string())?;
    let _ = c.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    c.write_all(format!("{q}\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut s = String::new();
    BufReader::new(c).read_line(&mut s).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(s.trim()).map_err(|e| e.to_string())?;
    if v.get("locked").is_some() {
        return Err("locked".into());
    }
    Ok(v)
}

/// The window in front: (its app id, its title).
pub fn front() -> (String, String) {
    if let Ok(v) = std::env::var("HEARTH_FILL_FRONT") {
        let (a, t) = v.split_once('|').unwrap_or(("", v.as_str()));
        return (a.into(), t.into());
    }
    let o = hearth_ui::run::blocking(&["hearth-mantel-ipc".to_string(), "call".into(), "pad".into(), "state".into()]);
    let v: Value = serde_json::from_str(o.out.trim()).unwrap_or(Value::Null);
    (v.get("front").and_then(Value::as_str).unwrap_or("").to_string(), v.get("title").and_then(Value::as_str).unwrap_or("").to_string())
}

pub fn is_browser(app: &str) -> bool {
    let a = app.to_lowercase();
    BROWSERS.iter().any(|b| a == *b || a.starts_with(&format!("{b}-")))
}

fn wtype_stdin(text: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut c = Command::new("wtype").args(["-d", "8", "-"]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_| "wtype isn't on this PC".to_string())?;
    if let Some(mut i) = c.stdin.take() {
        i.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    let s = c.wait().map_err(|e| e.to_string())?;
    if s.success() { Ok(()) } else { Err("the keyboard didn't take it".into()) }
}

/// Types the login into the window in front: user name, Tab, password (no Enter: you press it).
pub fn type_login(id: &str) -> Result<(), String> {
    let v = ask(&json!({"get": id}))?;
    let mut user = v.get("user").and_then(Value::as_str).unwrap_or("").to_string();
    let mut pw = v.get("password").and_then(Value::as_str).unwrap_or("").to_string();
    let r = (|| {
        if !user.is_empty() {
            wtype_stdin(&user)?;
            let _ = hearth_ui::run::blocking(&["wtype".to_string(), "-k".into(), "Tab".into()]);
        }
        wtype_stdin(&pw)
    })();
    user.zeroize();
    pw.zeroize();
    r
}

pub fn notify(title: &str, body: &str) {
    if std::env::var_os("HEARTH_FILL_FRONT").is_some() {
        eprintln!("{title}: {body}");
    }
    hearth_ui::run::detached(&["notify-send", "-a", "Hearth Passwords", title, body]);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(name: &str, uri: &str) -> Login {
        Login { id: name.into(), name: name.into(), uri: uri.into(), user: "me".into(), password: "pw".into() }
    }
    #[test]
    fn the_logins_that_fit_a_window() {
        assert_eq!(site_word("https://github.com/login"), "github");
        assert_eq!(site_word("accounts.google.com"), "google");
        assert_eq!(site_word("https://www.bbc.co.uk"), "bbc");
        let v = vec![l("GitHub", "https://github.com"), l("Steam", "https://store.steampowered.com"), l("Hearth AI", "")];
        let f = fits("Sign in to GitHub · GitHub — Mozilla Firefox", &v);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "GitHub");
        assert_eq!(fits("Hearth AI", &v)[0].name, "Hearth AI");
        assert!(fits("Some page", &v).is_empty());
        assert!(is_browser("firefox") && is_browser("google-chrome") && !is_browser("hearth-ai"));
    }
}

// ------------------------------------------------------------------ `--fill`: Super + Alt + P

use hearth_ui::ui::{self as hui, t, tb};
use hearth_ui::Pal;
use iced::widget::{column, container, mouse_area, row, Column, Space};
use iced::{Element, Length, Subscription, Task};

#[derive(Debug, Clone)]
pub enum F {
    Pick(String),
    Typed(Result<(), String>),
    Cancel,
    Key(iced::keyboard::Event),
}

pub struct Chooser {
    p: Pal,
    site: String,
    items: Vec<(String, String, String)>,
    lit: usize,
    said: String,
}

fn fill_update(c: &mut Chooser, m: F) -> Task<F> {
    match m {
        F::Pick(id) => {
            // the window closes first, so the keys go to the window you were in
            let (tx, rx) = futures::channel::oneshot::channel();
            let _ = std::thread::Builder::new().name("hearth-passwords-type".into()).spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(450));
                let _ = tx.send(type_login(&id));
            });
            return iced::window::latest().and_then(|w| iced::window::minimize(w, true)).chain(Task::perform(async move { rx.await.unwrap_or_else(|_| Err("it stopped".into())) }, F::Typed));
        }
        F::Typed(r) => {
            if let Err(e) = r {
                notify("Couldn't fill it in", &e);
            }
            return iced::exit();
        }
        F::Cancel => return iced::exit(),
        F::Key(ev) => {
            use iced::keyboard::{key::Named, Event as KE, Key};
            if let KE::KeyPressed { key, .. } = ev {
                match key.as_ref() {
                    Key::Named(Named::Escape) => return iced::exit(),
                    Key::Named(Named::ArrowDown) => c.lit = (c.lit + 1).min(c.items.len().saturating_sub(1)),
                    Key::Named(Named::ArrowUp) => c.lit = c.lit.saturating_sub(1),
                    Key::Named(Named::Enter) => {
                        if let Some(i) = c.items.get(c.lit) {
                            return fill_update(c, F::Pick(i.0.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Task::none()
}

fn fill_view(c: &Chooser) -> Element<'_, F> {
    let p = c.p;
    let mut col = Column::new().spacing(6.0).push(tb(format!("Fill in {}?", if c.site.is_empty() { "this window" } else { c.site.as_str() }), 16.0, p.text)).push(t("Hearth Passwords types the user name and password into the window you were in.", 12.0, p.muted));
    for (i, (id, name, user)) in c.items.iter().enumerate() {
        let lit = i == c.lit;
        let r = container(row![column![tb(name.clone(), 13.0, p.text), t(user.clone(), 12.0, p.muted)].spacing(2.0), Space::new().width(Length::Fill), t(if lit { "Enter" } else { "" }, 11.0, p.accent)].align_y(iced::Center))
            .padding(10.0)
            .width(Length::Fill)
            .style(hui::boxed(if lit { Pal::mix(p.panel2, p.accent, 0.18) } else { p.panel2 }, 8.0, Some((if lit { p.accent } else { p.line }, 1.0))));
        col = col.push(mouse_area(r).on_press(F::Pick(id.clone())).interaction(iced::mouse::Interaction::Pointer));
    }
    if !c.said.is_empty() {
        col = col.push(t(c.said.clone(), 12.0, p.muted));
    }
    col = col.push(row![Space::new().width(Length::Fill), mouse_area(t("Cancel (Esc)", 12.0, p.muted)).on_press(F::Cancel)]);
    container(col).padding(18.0).width(Length::Fill).height(Length::Fill).style(hui::boxed(p.bg, 0.0, None)).into()
}

/// `hearth-passwords-app --fill`: the window in front, the logins that fit it, typed in (asked first when you want).
pub fn run_fill() -> iced::Result {
    let prefs = hearth_oss::prefs::Prefs::load("passwords");
    let (app_id, title) = front();
    if !is_browser(&app_id) && !prefs.b("fillApps", true) {
        notify("Not filled in", "Filling in is for browsers only (Hearth Passwords › Filling it in › Also for apps).");
        return Ok(());
    }
    let found = match ask(&json!({"find": title})) {
        Ok(v) => v.get("items").and_then(Value::as_array).cloned().unwrap_or_default(),
        Err(_) => {
            notify("Open your vault first", "Open Hearth Passwords and unlock it; then Super + Alt + P fills in.");
            hearth_ui::run::detached(&["hearth-passwords-app"]);
            return Ok(());
        }
    };
    if found.is_empty() {
        notify("Nothing to fill in", &format!("No login in your vault fits \"{}\".", title.chars().take(60).collect::<String>()));
        return Ok(());
    }
    let items: Vec<(String, String, String)> = found.iter().map(|x| (x["id"].as_str().unwrap_or("").to_string(), x["name"].as_str().unwrap_or("").to_string(), x["user"].as_str().unwrap_or("").to_string())).collect();
    if !prefs.b("fillAsk", true) && items.len() == 1 && !hearth_ui::app::flag("--ask") {
        if let Err(e) = type_login(&items[0].0) {
            notify("Couldn't fill it in", &e);
        }
        return Ok(());
    }
    let site = found.first().and_then(|x| x["site"].as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string()).unwrap_or_default();
    let n = items.len();
    let w = hearth_ui::app::window("hearth-passwords-fill", (460.0, 150.0 + 62.0 * n.min(5) as f32), (360.0, 200.0));
    iced::application(move || (Chooser { p: hearth_ui::theme::load(), site: site.clone(), items: items.clone(), lit: 0, said: String::new() }, Task::none()), fill_update, fill_view)
        .title(|_: &Chooser| "Fill in".to_string())
        .subscription(|_: &Chooser| Subscription::batch([iced::keyboard::listen().map(F::Key)]))
        .theme(|c: &Chooser| hearth_ui::app::iced_theme(&c.p))
        .window(w)
        .default_font(hearth_ui::app::INTER)
        .run()
}
