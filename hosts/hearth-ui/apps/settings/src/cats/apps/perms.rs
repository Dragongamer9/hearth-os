//! Apps › App permissions (Mantel's SetAppPermissions.qml, KDE's Flatpak Permissions page in Hearth's look): pick an
//! app from Flathub and see what it may reach (the internet, sound, your files, devices, Bluetooth, its windows), as
//! it asks and as you changed it; a switch is Flatpak's own override for you, Reset takes your changes out
//! (hearth-flatpak-perms). The app sees it the next time it starts. `app-permissions:<app id>` opens at that app.

use super::kit::*;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{t, tb, G};
use iced::widget::{column, row};
use iced::{Length, Task};
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub apps: Option<Vec<Value>>,
    pub pick: String,
    pub want: String,
    pub cur: Value,
    pub choosing: bool,
    pub say: Say,
}

#[derive(Debug, Clone)]
pub enum M {
    Apps(Res),
    Choosing,
    Choose(String),
    Got(Res),
    Set(String, bool),
    Reset,
    Saved(String, Res),
}

fn fp(args: &[&str]) -> Vec<String> {
    py("hearth-flatpak-perms", args, 30)
}

pub fn open(sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.want = std::mem::take(&mut sh.part);
    read(fp(&["list"]), M::Apps)
}

pub fn update(_sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Apps(r) => {
            if !r.ok {
                st.apps = Some(Vec::new());
                st.say.bad(r.err);
                return Task::none();
            }
            let apps = ga(&r.v, "apps").to_vec();
            let first = if !st.want.is_empty() && apps.iter().any(|a| gs(a, "id") == st.want) { st.want.clone() } else { apps.first().map(|a| gs(a, "id")).unwrap_or_default() };
            st.apps = Some(apps);
            st.want.clear();
            if !first.is_empty() {
                return update(_sh, st, M::Choose(first));
            }
        }
        M::Choosing => st.choosing = !st.choosing,
        M::Choose(id) => {
            st.choosing = false;
            st.pick = id.clone();
            st.cur = Value::Null;
            return read(fp(&["get", &id]), M::Got);
        }
        M::Got(r) => {
            if r.ok {
                st.cur = r.v
            } else {
                st.say.bad(r.err)
            }
        }
        M::Set(key, on) => {
            let short = st.pick.rsplit('.').next().unwrap_or("").to_string();
            return read(fp(&["set", &st.pick, &key, if on { "on" } else { "off" }]), move |r| M::Saved(format!("Saved. {short} sees it the next time it starts."), r));
        }
        M::Reset => return read(fp(&["reset", &st.pick]), |r| M::Saved("Reset.".into(), r)),
        M::Saved(said, r) => {
            if r.ok {
                st.cur = r.v;
                st.say.ok(said);
            } else {
                st.say.bad(r.err);
            }
        }
    }
    Task::none()
}

fn icon(k: &str) -> G {
    match k {
        "network" => G::Link,
        "pulseaudio" => G::Sound,
        "devices" => G::Pad,
        "dri" => G::Display,
        "bluetooth" => G::Bt,
        "wayland" | "x11" => G::Grid,
        _ => G::Folder,
    }
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let mut v: Vec<E<M>> = Vec::new();
    let apps = st.apps.as_deref().unwrap_or(&[]);
    if st.apps.is_some() && apps.is_empty() {
        v.push(r(G::Store, "No apps from Flathub yet", "Apps from Flathub show here, each with what it may reach").view(&p));
    }
    if st.apps.is_none() {
        v.push(words(&p, "Looking at your apps…", false));
    }
    if !apps.is_empty() {
        let name = apps.iter().find(|a| gs(a, "id") == st.pick).map(|a| gs(a, "name")).unwrap_or_else(|| st.pick.clone());
        let changed = ga(&st.cur, "perms").iter().filter(|x| gb(x, "changed")).count();
        let sub = if changed > 0 { format!("{} · {changed} changed by you", st.pick) } else { st.pick.clone() };
        let top = row![icon_box(&p, G::Store), column![tb(name.clone(), sz(18.0), p.text), t(sub, sz(12.0), p.muted)].spacing(2.0).width(Length::Fill), pill(&p, format!("App: {name}"), true, Some(M::Choosing))].spacing(16.0).align_y(iced::Center);
        v.push(card(&p, top));
        if st.choosing {
            for a in apps {
                let id = gs(a, "id");
                v.push(r(G::Store, gs(a, "name"), id.clone()).value(if id == st.pick { "Shown" } else { "" }).indent().on(M::Choose(id)).view(&p));
            }
        }
    }
    if !st.cur.is_null() {
        let mut g: Vec<E<M>> = Vec::new();
        let perms = ga(&st.cur, "perms");
        for x in perms {
            let key = gs(x, "key");
            let now = gb(x, "now");
            let ch = gb(x, "changed");
            let sub = if ch { format!("{} · the app asks for it {}", gs(x, "sub"), if gb(x, "app") { "on" } else { "off" }) } else { gs(x, "sub") };
            let mut ctl = iced::widget::Row::new().spacing(10.0).align_y(iced::Center);
            if ch {
                ctl = ctl.push(chip_of(&p, "Changed", Chip::New));
            }
            ctl = ctl.push(switch(&p, now, Some(M::Set(key.clone(), !now))));
            g.push(srow(&p, icon(&key), gs(x, "label"), sub, ctl.into()));
        }
        if perms.iter().any(|x| gb(x, "changed")) {
            g.push(r(G::Update, "Reset to what the app asks for", "Takes out every change you made for this app").on(M::Reset).view(&p));
        }
        v.push(group(&p, "What it may reach", g));
    }
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}
