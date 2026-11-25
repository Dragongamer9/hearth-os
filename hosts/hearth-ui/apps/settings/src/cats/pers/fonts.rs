//! Fonts, cursor & icons (SetFonts.qml): the font and its size, the fixed-width font, the small and window title
//! fonts, smoothing, hinting and DPI, the cursor and its size, the icon theme. Saved by hearth-lookset (one file,
//! ~/.config/nimbus/fonts-cursor-icons.json, only what you chose; "Hearth's default" takes it out), which hands them
//! to GTK apps, fontconfig and the cursor. On KDE Plasma (Fedora edition) KDE's own settings get them too, with
//! KDE's tools: kwriteconfig6 (kdeglobals' fonts, smoothing; kcmfonts' DPI), plasma-apply-cursortheme and
//! plasma-changeicons, as KDE's System Settings does.

use super::kit::{self, dropdown, irow, irow_if, small, Ic};
use super::{kde, Msg, State};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::run;
use hearth_ui::ui::{self, t, t1, G};
use iced::widget::{column, container, Column};
use iced::{Element, Font, Length, Task};
use serde_json::{json, Value};

type E<'a> = Element<'a, Msg>;

fn e(st: &State) -> Value {
    st.fonts.as_ref().and_then(|f| f.get("effective")).cloned().unwrap_or(json!({}))
}
fn s(st: &State) -> Value {
    st.fonts.as_ref().and_then(|f| f.get("saved")).cloned().unwrap_or(json!({}))
}
fn def(st: &State, k: &str, d: Value) -> Value {
    st.fonts.as_ref().and_then(|f| f.get("defaults")).and_then(|x| x.get(k)).cloned().unwrap_or(d)
}

/// A font by its family name (iced finds it among the system's fonts). Names are kept once each.
fn font_named(name: &str) -> Font {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static NAMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);
    let mut g = NAMES.lock().unwrap();
    let m = g.get_or_insert_with(HashMap::new);
    if m.len() > 64 && !m.contains_key(name) {
        return hearth_ui::app::INTER;
    }
    let n: &'static str = *m.entry(name.to_string()).or_insert_with(|| Box::leak(name.to_string().into_boxed_str()));
    Font::with_name(n)
}

fn kq(v: &str) -> String {
    crate::data::q(v)
}
fn kw(file: &str, group: &str, key: &str, v: &str) -> String {
    format!("kwriteconfig6 --notify --file {file} --group {group} --key {key} {}", kq(v))
}
/// KDE's side of a change (Plasma), from what hearth-lookset says is in effect now.
fn kde_cmd(key: &str, e: &Value, dark: bool) -> Option<Vec<String>> {
    let f = |fam: &str, size: &str, w: u32| format!("{},{},-1,5,{w},0,0,0,0,0", kit::s_of(e, fam), kit::n_of(e, size, 10.0));
    let script = match key {
        "font" | "fontSize" => kw("kdeglobals", "General", "font", &f("font", "fontSize", 400)),
        "mono" | "monoSize" => kw("kdeglobals", "General", "fixed", &f("mono", "monoSize", 400)),
        "smallFont" | "smallSize" => kw("kdeglobals", "General", "smallestReadableFont", &f("smallFont", "smallSize", 400)),
        "titleFont" | "titleSize" => format!("{} && dbus-send --session --type=method_call --dest=org.kde.KWin /KWin org.kde.KWin.reconfigure", kw("kdeglobals", "WM", "activeFont", &f("titleFont", "titleSize", 700))),
        "antialias" | "subpixel" => {
            let aa = kit::s_of(e, "antialias");
            let sp = if aa == "rgba" { kit::s_of(e, "subpixel") } else { "none".into() };
            format!("{} && {}", kw("kdeglobals", "General", "XftAntialias", if aa == "none" { "false" } else { "true" }), kw("kdeglobals", "General", "XftSubPixel", &sp))
        }
        "hinting" => {
            let h = kit::s_of(e, "hinting");
            format!("{} && {}", kw("kdeglobals", "General", "XftHinting", if h == "none" { "false" } else { "true" }), kw("kdeglobals", "General", "XftHintStyle", &format!("hint{}", if h.is_empty() { "slight" } else { &h })))
        }
        "dpi" => {
            let d = kit::n_of(e, "dpi", 96.0) as i64;
            if d == 96 { "kwriteconfig6 --notify --file kcmfonts --group General --key forceFontDPI --delete".into() } else { kw("kcmfonts", "General", "forceFontDPI", &d.to_string()) }
        }
        "cursor" => format!("plasma-apply-cursortheme {}", kq(&kit::s_of(e, "cursor"))),
        "icons" => {
            let i = { let x = kit::s_of(e, "icons"); if x.is_empty() { (if dark { "breeze-dark" } else { "breeze" }).to_string() } else { x } };
            format!("/usr/libexec/plasma-changeicons {}", kq(&i))
        }
        _ => return None,
    };
    Some(kit::sh_t(&script, 20))
}

fn pick(st: &mut State, key: &'static str, v: Value) -> Task<Msg> {
    st.msg.clear();
    st.pick = None;
    let mut req = serde_json::Map::new();
    req.insert(key.into(), if v.as_str() == Some("") { Value::Null } else { v });
    Task::perform(kit::run_in(kit::job("hearth-lookset", &["set"]), Value::Object(req).to_string()), move |o| Msg::FontSet(key, o))
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Fonts(o) => match kit::answer(&o) {
            Ok(v) if v.is_object() => st.fonts = Some(v),
            Ok(_) => {}
            Err(er) => {
                st.msg = er;
                st.msg_bad = true;
            }
        },
        Msg::FontPick(k, v) => return pick(st, k, v),
        Msg::FontSize(k, v) => {
            st.sizes = Some((k, ((6.0 + v as f64 * 18.0) * 2.0).round() / 2.0));
            if !st.sizes_due {
                st.sizes_due = true;
                return Task::perform(run::after(700), |_| Msg::FontFlush);
            }
        }
        Msg::FontFlush => {
            st.sizes_due = false;
            if let Some((k, v)) = st.sizes {
                let d = def(st, k, json!(if k == "smallSize" { 8 } else { 10 })).as_f64().unwrap_or(10.0);
                return pick(st, k, if (v - d).abs() < 0.01 { json!("") } else { json!(v) });
            }
        }
        Msg::FontSet(k, o) => match kit::answer(&o) {
            Ok(r) => {
                if let Some(f) = st.fonts.as_mut() {
                    if let Some(x) = r.get("saved") {
                        f["saved"] = x.clone();
                    }
                    if let Some(x) = r.get("effective") {
                        f["effective"] = x.clone();
                    }
                }
                if st.sizes.map(|x| x.0) == Some(k) && !st.sizes_due {
                    st.sizes = None;
                }
                st.msg = match k {
                    "cursor" => "Saved. The cursor changes in apps as they open; sign out and in for everything.".into(),
                    "cursorSize" => "Saved. Sign out and in for every app to use it.".into(),
                    _ => "Saved. Apps opened from now on use it.".into(),
                };
                st.msg_bad = false;
                if kit::on_plasma() {
                    if let Some(cmd) = kde_cmd(k, &e(st), sh.p.dark) {
                        return kde(cmd);
                    }
                }
            }
            Err(er) => {
                st.msg = er;
                st.msg_bad = true;
            }
        },
        Msg::SmallOpen => st.small_open = !st.small_open,
        Msg::SmoothOpen => st.smooth_open = !st.smooth_open,
        _ => {}
    }
    Task::none()
}

fn size_row<'a>(p: &hearth_ui::Pal, st: &State, i: Ic, title: &str, key: &'static str, d: f64) -> E<'a> {
    let shown = match st.sizes {
        Some((k, v)) if k == key => v,
        _ => kit::n_of(&e(st), key, d),
    };
    let ok = st.fonts.is_some();
    let ctl: E<'a> = if ok { kit::slide(p, ((shown - 6.0) / 18.0) as f32, 0.0..=1.0, 1.0 / 36.0, format!("{shown} pt"), move |v| Msg::FontSize(key, v)) } else { t1(format!("{shown} pt"), sz(13.0), p.muted).into() };
    irow_if(p, ok, i, title, "", ctl)
}
fn dd_row<'a>(p: &hearth_ui::Pal, st: &State, i: Ic, title: &str, sub: &str, key: &'static str, default_label: &str) -> E<'a> {
    let cur = kit::s_of(&s(st), key);
    let label = if cur.is_empty() { format!("Hearth's default ({default_label})") } else { name_in(st, key, &cur) };
    let ok = st.fonts.is_some();
    irow_if(p, ok, i, title, sub, dropdown(p, ui::fit(&label, 34), if ok { Some(Msg::Open(key)) } else { None }))
}
fn list_key(key: &str) -> &'static str {
    match key {
        "mono" => "monoFonts",
        "cursor" => "cursors",
        "icons" => "icons",
        _ => "fonts",
    }
}
/// The list's entries: (value, label). A list holds names, or {id, name}.
fn entries(st: &State, key: &str) -> Vec<(String, String)> {
    st.fonts.as_ref().map(|f| kit::arr(f, list_key(key)).into_iter().map(|x| match x { Value::String(s) => (s.clone(), s.clone()), o => (kit::s_of(o, "id"), { let n = kit::s_of(o, "name"); if n.is_empty() { kit::s_of(o, "id") } else { n } }) }).collect()).unwrap_or_default()
}
fn name_in(st: &State, key: &str, v: &str) -> String {
    entries(st, key).into_iter().find(|x| x.0 == v).map(|x| x.1).unwrap_or_else(|| v.to_string())
}
fn default_label(st: &State, key: &str) -> String {
    match key {
        "font" => def(st, "font", json!("Noto Sans")).as_str().unwrap_or("Noto Sans").to_string(),
        "mono" => def(st, "mono", json!("Hack")).as_str().unwrap_or("Hack").to_string(),
        "smallFont" => def(st, "smallFont", json!("Noto Sans")).as_str().unwrap_or("Noto Sans").to_string(),
        "titleFont" => def(st, "titleFont", json!("Noto Sans")).as_str().unwrap_or("Noto Sans").to_string(),
        "cursor" => "Breeze".into(),
        _ => "follows light or dark".into(),
    }
}

pub fn page(a: &App) -> E<'_> {
    let st = &a.pers;
    let p = a.sh.p;
    let ev = e(st);
    let mut c = crate::widgets::Secs::on(a, st.skip);
    if st.fonts.is_none() {
        c = c.push(small(&p, if st.msg.is_empty() { "Reading your fonts…" } else { "" }));
    }
    c = c.push(head(&p, "Fonts"));
    c = c.push(dd_row(&p, st, Ic::Text, "Font", "Menus, buttons and text in apps", "font", &default_label(st, "font")));
    c = c.push(size_row(&p, st, Ic::Text, "Font size", "fontSize", 10.0));
    c = c.push(dd_row(&p, st, Ic::Kbd, "Fixed-width font", "Terminals and code", "mono", &default_label(st, "mono")));
    c = c.push(size_row(&p, st, Ic::Kbd, "Fixed-width font size", "monoSize", 10.0));
    let sm = { let x = kit::s_of(&ev, "smallFont"); if x.is_empty() { "Noto Sans".to_string() } else { x } };
    let ti = { let x = kit::s_of(&ev, "titleFont"); if x.is_empty() { "Noto Sans".to_string() } else { x } };
    let (sms, tis) = (kit::n_of(&ev, "smallSize", 8.0), kit::n_of(&ev, "titleSize", 10.0));
    c = c.push(kit::ilink(&p, Ic::Text, "Small and window title fonts", "Tooltips and captions, and the title bars of windows", format!("{sm} {sms} · {ti} {tis}"), Msg::SmallOpen));
    if st.small_open {
        let mut inner = Column::new().spacing(8.0);
        inner = inner.push(dd_row(&p, st, Ic::Text, "Small font", "Tooltips, captions and the smallest text", "smallFont", &default_label(st, "smallFont")));
        inner = inner.push(size_row(&p, st, Ic::Text, "Small font size", "smallSize", 8.0));
        inner = inner.push(dd_row(&p, st, Ic::Window, "Window title font", "The title bars of windows (KWin's frames, KDE and GNOME apps)", "titleFont", &default_label(st, "titleFont")));
        inner = inner.push(size_row(&p, st, Ic::Window, "Window title size", "titleSize", 10.0));
        // How the two look, written in them.
        let px = |pt: f64| (pt * 96.0 / 72.0) as f32;
        inner = inner.push(container(column![
            t(format!("Window title in {ti}"), px(tis), p.text).font(Font { weight: iced::font::Weight::Bold, ..font_named(&ti) }),
            t(format!("A small caption in {sm}"), px(sms), p.muted).font(font_named(&sm)),
        ].spacing(6.0)).padding(18.0).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None)));
        c = c.push(kit::indent(inner));
    }
    c = c.push(kit::ilink(&p, Ic::G(G::Gear), "Font smoothing and DPI", "For sharp or soft text on your screen", "", Msg::SmoothOpen));
    if st.smooth_open {
        let ok = st.fonts.is_some();
        let aa = { let x = kit::s_of(&ev, "antialias"); if x.is_empty() { "grayscale".to_string() } else { x } };
        let sp = { let x = kit::s_of(&ev, "subpixel"); if x.is_empty() { "rgb".to_string() } else { x } };
        let hi = { let x = kit::s_of(&ev, "hinting"); if x.is_empty() { "slight".to_string() } else { x } };
        let dpi = kit::n_of(&ev, "dpi", 96.0) as i64;
        let mut inner = Column::new().spacing(8.0);
        let opts = |list: &[(&str, &str)], cur: &str, key: &'static str, on: bool| seg(&p, list.iter().map(|(l, v)| (l.to_string(), cur == *v, if on && ok { Some(Msg::FontPick(key, json!(if key == "subpixel" && *v == "rgb" { "" } else { *v }))) } else { None })).collect());
        inner = inner.push(irow(&p, Ic::G(G::Gear), "Smoothing", "", opts(&[("Normal", "grayscale"), ("Subpixel (LCD)", "rgba"), ("Off", "none")], &aa, "antialias", true)));
        let lcd = aa == "rgba";
        inner = inner.push(irow_if(&p, lcd, Ic::G(G::Gear), "Sub-pixel order", if lcd { "Look at a zoomed letter: pick the one with no coloured fringes" } else { "Used when smoothing is Subpixel (LCD)" }, opts(&[("RGB", "rgb"), ("BGR", "bgr"), ("V-RGB", "vrgb"), ("V-BGR", "vbgr")], &sp, "subpixel", lcd)));
        inner = inner.push(irow(&p, Ic::G(G::Gear), "Hinting", "", opts(&[("None", "none"), ("Slight", "slight"), ("Medium", "medium"), ("Full", "full")], &hi, "hinting", true)));
        inner = inner.push(irow(&p, Ic::G(G::Gear), "Font DPI", "Bigger text without changing the display's scale", seg(&p, [("96 (normal)", 96i64), ("108", 108), ("120", 120), ("144", 144)].iter().map(|(l, v)| (l.to_string(), dpi == *v, if ok { Some(Msg::FontPick("dpi", if *v == 96 { json!("") } else { json!(v) })) } else { None })).collect())));
        c = c.push(kit::indent(inner));
    }
    c = c.push(head(&p, "Cursor"));
    c = c.push(dd_row(&p, st, Ic::Mouse, "Cursor", "", "cursor", "Breeze"));
    let cs = kit::n_of(&ev, "cursorSize", 24.0) as i64;
    let ok = st.fonts.is_some();
    c = c.push(irow_if(&p, ok, Ic::Mouse, "Cursor size", "", seg(&p, [("Normal", 24i64), ("Large", 32), ("Larger", 48), ("Huge", 64)].iter().map(|(l, v)| (l.to_string(), cs == *v, if ok { Some(Msg::FontPick("cursorSize", if *v == 24 { json!("") } else { json!(v) })) } else { None })).collect())));
    c = c.push(head(&p, "Icons"));
    c = c.push(dd_row(&p, st, Ic::G(G::Grid), "Icon theme", "App icons in Qt/KDE and GTK apps (Hearth's own icons stay)", "icons", "follows light or dark"));
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    c.into()
}

pub fn dialog<'a>(a: &'a App, id: &str) -> Option<E<'a>> {
    let st = &a.pers;
    let p = a.sh.p;
    let key: &'static str = match id {
        "font" => "font",
        "mono" => "mono",
        "smallFont" => "smallFont",
        "titleFont" => "titleFont",
        "cursor" => "cursor",
        "icons" => "icons",
        _ => return None,
    };
    let cur = kit::s_of(&s(st), key);
    let mut items = vec![(format!("Hearth's default ({})", default_label(st, key)), String::new(), cur.is_empty(), Msg::FontPick(key, json!("")))];
    for (v, l) in entries(st, key) {
        let on = v == cur;
        items.push((l, String::new(), on, Msg::FontPick(key, json!(v))));
    }
    let title = match key {
        "font" => "Font",
        "mono" => "Fixed-width font",
        "smallFont" => "Small font",
        "titleFont" => "Window title font",
        "cursor" => "Cursor",
        _ => "Icon theme",
    };
    Some(kit::pick_dialog(&p, title, items, Msg::Close))
}
