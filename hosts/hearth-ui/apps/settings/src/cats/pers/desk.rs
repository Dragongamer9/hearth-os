//! Bar and dock (SetBarDock.qml) and Wallpaper per screen, slideshow & picture of the day (SetWallpaperMore.qml).
//!
//! Fedora edition: the bar is KDE's panel and the wallpapers are Plasma's, so these are read and changed through
//! plasmashell's own scripting (panels(), desktops(): placement, auto-hide, floating, width, the app launcher, the
//! network speed widget; each screen's picture, Plasma's slideshow and its picture of the day from Bing, NASA or
//! Wikimedia Commons). Hearth's own keys (barStyle, dockMode, barEdge, wallScreens, wallShow, wallPotd, wallPair…)
//! are saved in desktop.json as well, the one record Mantel shares, so a look saved in My looks keeps them.
//! Off Plasma (Mantel), only desktop.json changes and Mantel shows it.

use super::kit::{self, iact, ilink, irow, small, Ic};
use super::{kde, save, For, Msg, State};
use crate::shared::Shared;
use crate::widgets::*;
use crate::App;
use hearth_ui::ui::{self, t, t1, ts, G};
use crate::widgets::btn;
use hearth_ui::Pal;
use iced::widget::{column, container, Row};
use iced::{Element, Length, Task};
use serde_json::{json, Value};

type E<'a> = Element<'a, Msg>;

const PANELS_JS: &str = r#"var o={panels:[]};var ps=panels();for(var i=0;i<ps.length;i++){var p=ps[i];var ws=p.widgets();var t=[];for(var j=0;j<ws.length;j++)t.push(ws[j].type);o.panels.push({id:p.id,location:p.location,height:p.height,hiding:p.hiding,floating:p.floating,lengthMode:p.lengthMode,alignment:p.alignment,opacity:p.opacity,screen:p.screen,widgets:t});}print(JSON.stringify(o));"#;
const WALLS_JS: &str = r#"var o={desktops:[],plugins:[]};try{o.plugins=Object.keys(knownWallpaperPlugins());}catch(e){}var ds=desktops();for(var i=0;i<ds.length;i++){var d=ds[i];if(d.screen<0)continue;var g=screenGeometry(d.screen);var w={screen:d.screen,plugin:d.wallpaperPlugin,x:g.x,y:g.y,width:g.width,height:g.height};d.currentConfigGroup=["Wallpaper","org.kde.image","General"];w.image=String(d.readConfig("Image",""));d.currentConfigGroup=["Wallpaper","org.kde.slideshow","General"];w.slidePaths=String(d.readConfig("SlidePaths",""));d.currentConfigGroup=["Wallpaper","org.kde.potd","General"];w.provider=String(d.readConfig("Provider",""));o.desktops.push(w);}print(JSON.stringify(o));"#;
/// Swapping the app launcher on panel `p` (KDE's scripting) for `__NEW__`, in the old one's place: the owner on the
/// Ally (Nov 19) "Switching app menus put the app menu on the right side": addWidget puts it at the end, and the old
/// code only moved it when the panel's AppletOrder was written down already. Now: the order (AppletOrder, or the
/// panel's widgets as they are), the old launcher's place in it (the start when there's none), the new one written
/// there and the old one removed, then the panel reads its order again. hosts/linux/test/test_plasma_panel_js.py runs
/// it against a stand-in panel.
pub const SWAP_LAUNCHER_JS: &str = r#"var L=["org.kde.plasma.kickoff","org.kde.plasma.kicker","org.kde.plasma.kickerdash","org.hearthos.appmenu"];var ws=p.widgets();var old=null;for(var i=0;i<ws.length;i++)if(L.indexOf(ws[i].type)>=0){old=ws[i];break;}p.currentConfigGroup=["General"];var ord=String(p.readConfig("AppletOrder",""));var ids=[];var parts=ord.split(";");for(var i=0;i<parts.length;i++)if(parts[i]!=="")ids.push(parts[i]);if(!ids.length)for(var i=0;i<ws.length;i++)ids.push(String(ws[i].id));var at=old?ids.indexOf(String(old.id)):0;if(at<0)at=0;var n=p.addWidget(__NEW__);if(n){var k=ids.indexOf(String(n.id));if(k>=0)ids.splice(k,1);if(old){var o=ids.indexOf(String(old.id));if(o>=0)ids.splice(o,1);old.remove();}if(at>ids.length)at=ids.length;ids.splice(at,0,String(n.id));p.currentConfigGroup=["General"];p.writeConfig("AppletOrder",ids.join(";"));p.reloadConfig();}"#;

const LAUNCHERS: &[(&str, &str, &str)] = &[
    ("org.kde.plasma.kickoff", "Application Launcher", "KDE's menu with favourites, search and categories"),
    ("org.kde.plasma.kicker", "Application Menu", "A classic list that opens from the corner"),
    ("org.kde.plasma.kickerdash", "Full screen", "Every app over the whole screen; easiest with a controller"),
];
const NETSPEED: &str = "org.kde.plasma.systemmonitor.net";

pub fn read_panels() -> Task<Msg> {
    if !kit::on_plasma() {
        return Task::none();
    }
    Task::perform(kit::run_in(kit::plasma(PANELS_JS), String::new()), Msg::Panels)
}
pub fn read_walls() -> Task<Msg> {
    if !kit::on_plasma() {
        return Task::none();
    }
    let cmd = if let Some(f) = std::env::var_os("HEARTH_PLASMA_FIXTURE") {
        vec!["cat".into(), f.to_string_lossy().to_string()]
    } else {
        let p = kit::plasma(WALLS_JS).iter().map(|a| crate::data::q(a)).collect::<Vec<_>>().join(" ");
        kit::sh_t(&format!("echo '#@plasma'; {p}; echo; echo '#@ks'; timeout 5 kscreen-doctor -j 2>/dev/null"), 20)
    };
    Task::perform(kit::run_in(cmd, String::new()), Msg::Walls)
}

fn panel_js(id: i64, body: &str) -> Vec<String> {
    kit::plasma_do(&format!("var p=panelById({id});if(p){{{body}}}"))
}
fn js_then(cmd: Vec<String>) -> Task<Msg> {
    Task::perform(kit::run_in(cmd, String::new()), Msg::JsDone)
}

// ---------------------------------------------------------------- the panel
fn panels(st: &State) -> Vec<Value> {
    st.panels.as_ref().map(|v| kit::arr(v, "panels").into_iter().cloned().collect()).unwrap_or_default()
}
fn cur_panel(st: &State) -> Option<Value> {
    let ps = panels(st);
    ps.get(st.panel.min(ps.len().saturating_sub(1))).cloned()
}
fn pid(pn: &Value) -> i64 {
    pn.get("id").and_then(Value::as_i64).unwrap_or(-1)
}
fn has_widget(pn: &Value, ty: &str) -> bool {
    kit::strs(pn, "widgets").iter().any(|w| w == ty)
}

/// Desktop clock and weather: Hearth's key, and on Plasma KDE's clock and weather widgets on each desktop.
pub fn desk_clock(sh: &mut Shared, v: String) -> Task<Msg> {
    let mut t = vec![save(sh, json!({"deskClock": v}))];
    if kit::on_plasma() {
        let js = match v.as_str() {
            "on" => r#"var ds=desktops();for(var i=0;i<ds.length;i++){var d=ds[i];if(d.screen<0)continue;var have={};var ws=d.widgets();for(var j=0;j<ws.length;j++)have[ws[j].type]=1;if(!have["org.kde.plasma.digitalclock"])d.addWidget("org.kde.plasma.digitalclock",48,48,320,160);if(!have["org.kde.plasma.weather"])d.addWidget("org.kde.plasma.weather",48,220,320,200);}"#,
            "off" => r#"var ds=desktops();for(var i=0;i<ds.length;i++){var ws=ds[i].widgets();for(var j=0;j<ws.length;j++){var t=ws[j].type;if(t=="org.kde.plasma.digitalclock"||t=="org.kde.plasma.weather")ws[j].remove();}}"#,
            _ => "",
        };
        if !js.is_empty() {
            t.push(kde(kit::plasma_do(js)));
        }
    }
    Task::batch(t)
}
/// Panel transparency on Plasma 6 (the owner, Nov 19: "Panel transparency doesn't work"): the scripting's
/// `opacity` where this Plasma has it (live), and always the panel's own line in plasmashellrc
/// ([PlasmaViews][Panel N] panelOpacity: 0 adaptive, 1 opaque, 2 translucent), which is what Plasma reads at its next
/// start, so it survives signing out (hearth-plasma-looks never writes it). Where the scripting has no `opacity`,
/// plasmashell is started again so the line takes effect now. KDE's own Plasma style decides how see-through
/// "translucent" looks (and needs KWin's blur for frosted).
pub fn opacity_cmd(o: &str) -> Vec<String> {
    if std::env::var_os("HEARTH_PLASMA_FIXTURE").is_some() {
        return vec!["true".into()];
    }
    let n = match o { "opaque" => 1, "translucent" => 2, _ => 0 };
    let js = format!("var ps=panels();var live=0;var ids=[];for(var i=0;i<ps.length;i++){{ids.push(ps[i].id);if(typeof ps[i].opacity!==\"undefined\"){{ps[i].opacity={};live=1;}}}}print((live?\"live\":\"file\")+\" \"+ids.join(\" \"));", kit::js_str(o));
    let script = format!(
        "out=$(timeout 10 dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript \"string:$1\" 2>/dev/null | sed -n 's/^ *string \"\\(.*\\)\"$/\\1/p'); \
         set -- $out; how=${{1:-file}}; [ $# -gt 0 ] && shift; \
         for id in \"$@\"; do kwriteconfig6 --file plasmashellrc --group PlasmaViews --group \"Panel $id\" --key panelOpacity {n}; done; \
         [ \"$how\" = live ] || {{ systemctl --user restart plasma-plasmashell.service 2>/dev/null || {{ kquitapp6 plasmashell; (kstart plasmashell >/dev/null 2>&1 &); }}; }}; echo \"$how\""
    );
    vec!["sh".into(), "-c".into(), script, "sh".into(), js]
}

/// Panel transparency: Hearth's key, and on Plasma the panel's own (adaptive, opaque, translucent).
pub fn glass(sh: &mut Shared, st: &mut State, v: String) -> Task<Msg> {
    let mut t = vec![save(sh, json!({"panelGlass": v}))];
    if kit::on_plasma() {
        let o = match v.as_str() { "solid" => "opaque", "frosted" | "clear" => "translucent", _ => "adaptive" };
        t.push(kde(opacity_cmd(o)));
        st.panels = None;
        t.push(read_panels());
    }
    Task::batch(t)
}

pub fn update(sh: &mut Shared, st: &mut State, m: Msg) -> Task<Msg> {
    match m {
        Msg::Panels(o) => {
            st.panel_err = kit::plasma_why(&o);
            st.panels = kit::plasma_json(&o);
            if st.panels.is_none() && st.panel_err.is_empty() {
                st.panel_err = "KDE's panel didn't answer".into();
            }
        }
        Msg::Panel(i) => {
            st.panel = i;
            st.panel_remove = false;
        }
        // Removing a panel asks once more first.
        Msg::RemovePanel => {
            if !st.panel_remove {
                st.panel_remove = true;
                return Task::none();
            }
            st.panel_remove = false;
            if let Some(pn) = cur_panel(st) {
                st.panel = 0;
                return js_then(panel_js(pid(&pn), "p.remove();"));
            }
        }
        Msg::Js(body) => {
            if let Some(pn) = cur_panel(st) {
                return js_then(panel_js(pid(&pn), &body));
            }
        }
        Msg::JsSet(body, v) => {
            let mut t = vec![save(sh, v)];
            if let Some(pn) = cur_panel(st) {
                t.push(js_then(panel_js(pid(&pn), &body)));
            }
            return Task::batch(t);
        }
        Msg::JsDone(o) => {
            if !o.ok {
                st.msg = format!("KDE's panel didn't take that: {}", kit::plasma_why(&o));
                st.msg_bad = true;
            }
            return Task::batch([read_panels(), read_walls()]);
        }
        // Hearth's panel on Plasma (hosts/linux/desktop/plasma-hearth): put in or taken off only when you pick it.
        Msg::PanelPick(k) => {
            return Task::perform(kit::run_in(vec!["hearth-plasma-panel".into(), k.into()], String::new()), Msg::JsDone);
        }
        Msg::EditMode => {
            return kde(vec!["timeout".into(), "5".into(), "dbus-send".into(), "--session".into(), "--dest=org.kde.plasmashell".into(), "--type=method_call".into(), "/PlasmaShell".into(), "org.freedesktop.DBus.Properties.Set".into(), "string:org.kde.PlasmaShell".into(), "string:editMode".into(), "variant:boolean:true".into()]);
        }
        Msg::Set(v) => return save(sh, v),
        Msg::DeskClock(v) => return desk_clock(sh, v),
        Msg::Glass(v) => return glass(sh, st, v),
        Msg::Walls(o) => {
            st.walls_err = kit::plasma_why(&o);
            let secs = crate::data::sections(&o.out);
            let plasma_out = run_like(o.ok, secs.get("plasma").cloned().unwrap_or_else(|| o.out.clone()));
            let mut v = kit::plasma_json(&plasma_out).unwrap_or(Value::Null);
            if let Some(ks) = secs.get("ks").and_then(|k| serde_json::from_str::<Value>(k).ok()) {
                if v.is_object() {
                    v["ks"] = ks;
                }
            }
            st.walls = if v.is_object() { Some(v) } else { None };
            if st.walls.is_none() && st.walls_err.is_empty() {
                st.walls_err = "Plasma's wallpapers didn't answer".into();
            }
        }
        Msg::Choose(f) => {
            if st.picking {
                return Task::none();
            }
            st.picking = true;
            let folder = f == For::Folder;
            let cmd = kit::file_picker(if folder { "Pick a folder of pictures" } else { "Choose a wallpaper" }, folder, "image/jpeg image/png image/webp");
            return Task::perform(kit::run_in(cmd, String::new()), move |o| Msg::Picked(f.clone(), o));
        }
        Msg::Picked(f, o) => {
            st.picking = false;
            if o.code == 3 {
                st.msg = "No file picker installed".into();
                st.msg_bad = true;
                return Task::none();
            }
            let path = o.out.trim().to_string();
            if path.is_empty() {
                return Task::none();
            }
            return match f {
                For::Screen(i) => set_screen(sh, st, i, path),
                For::Folder => set_show(sh, st, "folder", json!(path)),
                For::Pair(which) => set_pair(sh, st, which, json!(path)),
                For::Wall => Task::none(),
            };
        }
        Msg::ScreenSame(i) => return set_screen(sh, st, i, String::new()),
        Msg::Show(k, v) => {
            let mut t = vec![set_show(sh, st, k, v.clone())];
            if k == "on" && v == json!(true) && kit::s_of(&Value::Object(kit::obj(sh, "wallShow")), "folder").is_empty() {
                t.push(update(sh, st, Msg::Choose(For::Folder)));
            }
            return Task::batch(t);
        }
        Msg::Potd(k, v) => {
            let mut o = json!({"on": false, "source": "bing"});
            for (a, b) in kit::obj(sh, "wallPotd") {
                o[a] = b;
            }
            o[k] = v;
            let t = save(sh, json!({"wallPotd": o}));
            return Task::batch([t, apply_walls(sh, st)]);
        }
        Msg::PotdAgain => return apply_walls(sh, st),
        Msg::Pair(k, v) => return set_pair(sh, st, k, v),
        _ => {}
    }
    Task::none()
}

fn run_like(ok: bool, out: String) -> hearth_ui::run::Output {
    hearth_ui::run::Output { ok, code: if ok { 0 } else { 1 }, out, err: String::new() }
}

// ---------------------------------------------------------------- wallpapers
/// The screens Plasma has, with their names (kscreen-doctor's, matched by where they are) and what they show.
#[derive(Clone)]
pub struct Screen {
    pub idx: i64,
    pub name: String,
    pub w: i64,
    pub h: i64,
    pub plugin: String,
    pub image: String,
}
pub fn screens(st: &State) -> Vec<Screen> {
    let Some(v) = st.walls.as_ref() else { return vec![] };
    let ks: Vec<Value> = v.get("ks").map(|k| kit::arr(k, "outputs").into_iter().cloned().collect()).unwrap_or_default();
    let mut out: Vec<Screen> = kit::arr(v, "desktops")
        .into_iter()
        .map(|d| {
            let (x, y) = (kit::n_of(d, "x", 0.0) as i64, kit::n_of(d, "y", 0.0) as i64);
            let idx = kit::n_of(d, "screen", 0.0) as i64;
            let name = ks
                .iter()
                .find(|o| o.get("pos").map(|p| kit::n_of(p, "x", -1.0) as i64 == x && kit::n_of(p, "y", -1.0) as i64 == y).unwrap_or(false) && o.get("enabled").and_then(Value::as_bool).unwrap_or(true))
                .map(|o| kit::s_of(o, "name"))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("Screen {}", idx + 1));
            Screen { idx, name, w: kit::n_of(d, "width", 0.0) as i64, h: kit::n_of(d, "height", 0.0) as i64, plugin: kit::s_of(d, "plugin"), image: kit::s_of(d, "image").trim_start_matches("file://").to_string() }
        })
        .collect();
    out.sort_by_key(|s| s.idx);
    out
}
fn has_plugin(st: &State, id: &str) -> bool {
    match st.walls.as_ref().and_then(|v| v.get("plugins")).and_then(Value::as_array) {
        Some(l) if !l.is_empty() => l.iter().any(|x| x.as_str() == Some(id)),
        _ => true,
    }
}

/// What every screen should show now, from desktop.json (a screen's own picture, else the picture of the day, else
/// the slideshow, else the wallpaper from Personalization), put on Plasma's desktops in one script.
fn walls_js(sh: &Shared, st: &State) -> String {
    let own = kit::obj(sh, "wallScreens");
    let show = Value::Object(kit::obj(sh, "wallShow"));
    let potd = Value::Object(kit::obj(sh, "wallPotd"));
    let base = super::looks::plasma_wall(sh, &sh.d_str("wallpaper"));
    let mut js = String::from("var ds=desktops();");
    for s in screens(st) {
        let mine = own.get(&s.name).and_then(Value::as_str).unwrap_or("");
        let set_img = |f: &str| format!("d.wallpaperPlugin=\"org.kde.image\";d.currentConfigGroup=[\"Wallpaper\",\"org.kde.image\",\"General\"];d.writeConfig(\"Image\",{});", kit::js_str(&format!("file://{f}")));
        let body = if !mine.is_empty() {
            set_img(mine)
        } else if kit::b_of(&potd, "on") && has_plugin(st, "org.kde.potd") {
            let prov = match kit::s_of(&potd, "source").as_str() { "apod" => "apod", "wikimedia" => "wcpotd", _ => "bing" };
            format!("d.wallpaperPlugin=\"org.kde.potd\";d.currentConfigGroup=[\"Wallpaper\",\"org.kde.potd\",\"General\"];d.writeConfig(\"Provider\",{});", kit::js_str(prov))
        } else if kit::b_of(&show, "on") && !kit::s_of(&show, "folder").is_empty() {
            let mins = kit::n_of(&show, "minutes", 30.0).max(1.0) as i64;
            format!("d.wallpaperPlugin=\"org.kde.slideshow\";d.currentConfigGroup=[\"Wallpaper\",\"org.kde.slideshow\",\"General\"];d.writeConfig(\"SlidePaths\",{});d.writeConfig(\"SlideInterval\",{});d.writeConfig(\"SlideshowMode\",{});", kit::js_str(&kit::s_of(&show, "folder")), mins * 60, if kit::b_of(&show, "shuffle") { 0 } else { 1 })
        } else {
            set_img(&base)
        };
        js.push_str(&format!("for(var i=0;i<ds.length;i++){{if(ds[i].screen=={}){{var d=ds[i];{body}}}}}", s.idx));
    }
    js
}
fn apply_walls(sh: &Shared, st: &mut State) -> Task<Msg> {
    if !kit::on_plasma() || st.walls.is_none() {
        return Task::none();
    }
    js_then(kit::plasma_do(&walls_js(sh, st)))
}
fn set_screen(sh: &mut Shared, st: &mut State, i: usize, f: String) -> Task<Msg> {
    let Some(s) = screens(st).get(i).cloned() else { return Task::none() };
    let mut m = kit::obj(sh, "wallScreens");
    if f.is_empty() {
        m.remove(&s.name);
    } else {
        m.insert(s.name.clone(), json!(f));
    }
    let t = save(sh, json!({"wallScreens": m}));
    Task::batch([t, apply_walls(sh, st)])
}
fn set_show(sh: &mut Shared, st: &mut State, k: &str, v: Value) -> Task<Msg> {
    let mut o = json!({"on": false, "folder": "", "minutes": 30, "shuffle": false});
    for (a, b) in kit::obj(sh, "wallShow") {
        o[a] = b;
    }
    o[k] = v;
    let t = save(sh, json!({"wallShow": o}));
    Task::batch([t, apply_walls(sh, st)])
}
fn set_pair(sh: &mut Shared, st: &mut State, k: &str, v: Value) -> Task<Msg> {
    let mut o = json!({"on": false, "light": "", "dark": ""});
    for (a, b) in kit::obj(sh, "wallPair") {
        o[a] = b;
    }
    o[k] = v;
    let mut t = vec![save(sh, json!({"wallPair": o.clone()}))];
    // On Plasma: the one for the mode on now, put on at once (a mode changed in Personalization puts on the other).
    if kit::on_plasma() && kit::b_of(&o, "on") {
        let md = if sh.d_str("mode") == "light" { "light" } else { "dark" };
        let f = kit::s_of(&o, md);
        if !f.is_empty() {
            t.push(kde(vec!["timeout".into(), "20".into(), "plasma-apply-wallpaperimage".into(), f]));
        }
    } else if kit::on_plasma() && k == "on" {
        t.push(apply_walls(sh, st));
    }
    Task::batch(t)
}

// ---------------------------------------------------------------- Bar and dock
pub fn bar_dock(a: &App) -> E<'_> {
    let sh = &a.sh;
    let st = &a.pers;
    let p = sh.p;
    let plasma = kit::on_plasma();
    let mut c = kit::Col::new(st.skip);
    c = c.push(t(if plasma { "On KDE Plasma the bar and dock are KDE's panel: set it here, and it stays whichever look you choose." } else { "Every look has its own bar and dock. Pick yours here and it stays, whichever look you choose." }, sz(14.0), p.muted));
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    c = c.push(head(&p, "Bar and dock"));
    let bs = sh.d_str("barStyle");
    // Hearth's panel (Mantel's bar, dock and app menus on Plasma, plasma-hearth) or KDE's own panel.
    let hearth_panel = plasma && panels(st).iter().any(|pn| has_widget(pn, "org.hearthos.panel"));
    if plasma && st.panels.is_some() {
        c = c.push(irow(&p, Ic::Window, "Panel", if hearth_panel { "Hearth's panel: the bar and dock from Hearth's Desktop Mode, in your look" } else { "KDE's panel. Hearth's panel is the bar and dock from Hearth's Desktop Mode, in your look (your panel is saved first)" }, seg(&p, vec![
            ("Hearth's panel".into(), hearth_panel, if hearth_panel { None } else { Some(Msg::PanelPick("apply")) }),
            ("KDE's panel".into(), !hearth_panel, if hearth_panel { Some(Msg::PanelPick("undo")) } else { None }),
        ])));
    }
    if plasma && !hearth_panel {
        let ps = panels(st);
        if st.panels.is_none() {
            c = c.push(small(&p, if st.panel_err.is_empty() { "Asking KDE's panel…".to_string() } else { format!("{}.", st.panel_err) }));
        }
        if ps.len() > 1 {
            let items = ps.iter().enumerate().map(|(i, pn)| (format!("Panel {} · {}", i + 1, kit::s_of(pn, "location")), i == st.panel, Some(Msg::Panel(i)))).collect();
            c = c.push(irow(&p, Ic::Window, "Panel", "Which of your panels these change", seg(&p, items)));
        }
        if let Some(pn) = cur_panel(st) {
            let main = st.panel == 0;
            let loc = kit::s_of(&pn, "location");
            let js_for = |body: String, key: &str, v: Value| if main { Msg::JsSet(body, json!({ key: v })) } else { Msg::Js(body) };
            c = c.push(irow(&p, Ic::G(G::Display), "Placement", "Which edge of the screen it sits on. Left and right stack the icons", seg(&p, ["bottom", "top", "left", "right"].iter().map(|v| (titlecase(v), loc == *v, Some(js_for(format!("p.location={};", kit::js_str(v)), "barEdge", json!(v))))).collect())));
            let hid = kit::s_of(&pn, "hiding");
            let hid = if hid.is_empty() { "none".to_string() } else { hid };
            c = c.push(irow(&p, Ic::Eye, "Auto-hide", "Auto-hide: it comes back when the pointer touches the edge, and leaves the whole screen to your windows", seg(&p, [("Always shown", "none"), ("Auto-hide", "autohide"), ("Dodge windows", "dodgewindows"), ("Windows go below", "windowsgobelow")].iter().map(|(l, v)| (l.to_string(), hid == *v, Some(js_for(format!("p.hiding={};", kit::js_str(v)), "dockMode", json!(if *v == "none" { "shown" } else { "autohide" }))))).collect())));
            let fl = pn.get("floating").and_then(Value::as_bool).unwrap_or(false);
            let lm = kit::s_of(&pn, "lengthMode");
            c = c.push(irow(&p, Ic::Window, "Bar style", "Floating islands sit off the edge with a gap; a full-width bar goes from side to side", seg(&p, vec![
                ("Floating islands".into(), fl, Some(Msg::JsSet(String::from("p.floating=true;"), json!({"barStyle": "floating"})))),
                ("Full-width bar".into(), !fl, Some(Msg::JsSet(String::from("p.floating=false;p.lengthMode=\"fill\";"), json!({"barStyle": "bar"})))),
            ])));
            c = c.push(irow(&p, Ic::G(G::Grid), "Width", "Fit to its icons makes it a dock in the middle", seg(&p, [("Whole edge", "fill"), ("Fit to its icons", "fit")].iter().map(|(l, v)| (l.to_string(), lm == *v || (lm.is_empty() && *v == "fill"), Some(Msg::Js(format!("p.lengthMode={};", kit::js_str(v)))))).collect())));
            if lm == "fit" || lm == "custom" {
                let al = kit::s_of(&pn, "alignment");
                c = c.push(irow(&p, Ic::G(G::Grid), "Where along the edge", "", seg(&p, [("Start", "left"), ("Middle", "center"), ("End", "right")].iter().map(|(l, v)| (l.to_string(), al == *v, Some(Msg::Js(format!("p.alignment={};", kit::js_str(v)))))).collect())));
            }
            let h = kit::n_of(&pn, "height", 44.0) as f32;
            c = c.push(irow(&p, Ic::Window, "Thickness", "", seg(&p, [("Thin", 32.0f32), ("Normal", 44.0), ("Thick", 56.0), ("Big", 72.0)].iter().map(|(l, v)| (l.to_string(), (h - v).abs() < 4.0, Some(Msg::Js(format!("p.height={};", *v as i64))))).collect())));
            let net = has_widget(&pn, NETSPEED);
            let ns_js = if net {
                format!("var w=p.widgets({});for(var i=0;i<w.length;i++)w[i].remove();", kit::js_str(NETSPEED))
            } else {
                // Next to the system tray (the network icon), as Mantel puts it beside the network.
                format!("p.currentConfigGroup=[\"General\"];var ord=String(p.readConfig(\"AppletOrder\",\"\"));var n=p.addWidget({});var tr=p.widgets(\"org.kde.plasma.systemtray\");if(n&&tr.length&&ord){{var ids=ord.split(\";\");var at=ids.indexOf(String(tr[0].id));if(at>=0){{ids.splice(at,0,String(n.id));p.currentConfigGroup=[\"General\"];p.writeConfig(\"AppletOrder\",ids.join(\";\"));p.reloadConfig();}}}}", kit::js_str(NETSPEED))
            };
            c = c.push(irow(&p, Ic::Wave, "Network speed in the bar", "How fast things download and upload right now, next to the network icon", switch(&p, net, Some(Msg::JsSet(ns_js, json!({"netSpeed": !net}))))));
            c = c.push(ilink(&p, Ic::G(G::Grid), "Arrange the bar", format!("Move the panel's items, take off the ones you don't use, add widgets ({} now)", if ps.len() == 1 { "1 panel".to_string() } else { format!("{} panels", ps.len()) }), "", Msg::EditMode));
            let mut r = Row::new().spacing(10.0).push(btn(&p, "Add a panel", ui::Kind::Normal, false, Some(Msg::Js(String::from("var np=new Panel(\"org.kde.panel\");np.location=\"top\";np.height=36;np.addWidget(\"org.kde.plasma.digitalclock\");")))));
            if ps.len() > 1 {
                r = r.push(btn(&p, if st.panel_remove { "Yes, remove this panel" } else { "Remove this panel" }, ui::Kind::Danger, false, Some(Msg::RemovePanel)));
            }
            c = c.push(kit::indent(r));
            // The app launcher on this panel: KDE's three, swapped in its place.
            c = c.push(head(&p, "App menu"));
            let cur = LAUNCHERS.iter().find(|l| has_widget(&pn, l.0)).map(|l| l.0).unwrap_or("");
            for (id, label, sub) in LAUNCHERS {
                let on = cur == *id;
                let swap = SWAP_LAUNCHER_JS.replace("__NEW__", &kit::js_str(id));
                c = c.push(irow(&p, Ic::Menu, *label, *sub, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", ui::Kind::Normal, false, Some(Msg::Js(swap))) }));
            }
            // Hearth's app menus (Mantel's five, org.hearthos.appmenu) in the same place; the style is Hearth's appMenu.
            let am = sh.d_str("appMenu");
            let hearth_on = has_widget(&pn, "org.hearthos.appmenu");
            let swap = SWAP_LAUNCHER_JS.replace("__NEW__", &kit::js_str("org.hearthos.appmenu"));
            for (v, l, d) in [("hearth", "Hearth", "Big centred panel with Apps and Games tabs"), ("centred", "Centred", "Pinned apps and Recommended, above the centred dock"), ("tiles", "Tiles", "App list and live tiles in the corner"), ("start", "Start screen", "Full-screen live tiles; easiest with a controller"), ("classic", "Classic", "Two columns with your places; glassy, from the corner")] {
                let on = hearth_on && (am == v || (am.is_empty() && v == "hearth"));
                let act = if hearth_on { Msg::Set(json!({"appMenu": v})) } else { Msg::JsSet(swap.clone(), json!({"appMenu": v})) };
                c = c.push(irow(&p, Ic::Menu, format!("Hearth's menu: {l}"), d, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", ui::Kind::Normal, false, Some(act)) }));
            }
            if cur.is_empty() && !hearth_on {
                c = c.push(small(&p, "This panel has no app launcher: Use puts one on it."));
            }
        }
    } else {
        // Mantel's: the layout, its edge, the floating dock, auto-hide, the bar style (desktop.json).
        let lay = sh.d_str("barLayout");
        let one = lay == "one";
        c = c.push(irow(&p, Ic::Window, "Layout", if one { "Everything in one bar, on the edge you choose" } else { "Mantel's own: islands at the top, your apps in a dock along the bottom" }, seg(&p, vec![("Top bar + floating dock".into(), !one, Some(Msg::Set(json!({"barLayout": "split"})))), ("One main bar".into(), one, Some(Msg::Set(json!({"barLayout": "one"}))))])));
        if one {
            let e = { let x = sh.d_str("barEdge"); if x.is_empty() { "bottom".to_string() } else { x } };
            c = c.push(irow(&p, Ic::G(G::Display), "Placement", "Where the one main bar goes. Left and right stack the icons; menus open toward the screen.", seg(&p, ["bottom", "top", "left", "right"].iter().map(|v| (titlecase(v), e == *v, Some(Msg::Set(json!({"barEdge": v}))))).collect())));
        }
        let df = sh.d_str("dockFloat") != "off";
        c = c.push(irow(&p, Ic::G(G::Grid), "Floating app menu", if df { "On: the app menu button and your apps float in a dock along the bottom" } else { "Off: the app menu button and your apps are part of the main bar" }, switch(&p, df, Some(Msg::Set(json!({"dockFloat": if df { "off" } else { "on" }}))))));
        let dm = sh.d_str("dockMode");
        c = c.push(irow(&p, Ic::Eye, "Auto-hide", "The dock comes back when the pointer touches the bottom edge.", seg(&p, vec![("Look's own".into(), dm.is_empty(), Some(Msg::Set(json!({"dockMode": ""})))), ("Shown".into(), dm == "shown", Some(Msg::Set(json!({"dockMode": "shown"})))), ("Auto-hide".into(), dm == "autohide", Some(Msg::Set(json!({"dockMode": "autohide"}))))])));
        c = c.push(irow(&p, Ic::Window, "Bar style", "", seg(&p, vec![("Look's own".into(), bs.is_empty(), Some(Msg::Set(json!({"barStyle": ""})))), ("Floating islands".into(), bs == "floating", Some(Msg::Set(json!({"barStyle": "floating"})))), ("Full-width bar".into(), bs == "bar", Some(Msg::Set(json!({"barStyle": "bar"}))))])));
        let ns = sh.d_bool("netSpeed", false);
        c = c.push(irow(&p, Ic::Wave, "Network speed in the bar", "How fast things download and upload right now, next to the network icon", switch(&p, ns, Some(Msg::Set(json!({"netSpeed": !ns}))))));
        c = c.push(head(&p, "App menu"));
        let am = sh.d_str("appMenu");
        for (v, l, d) in [("hearth", "Hearth", "Big centred panel with Apps and Games tabs"), ("centred", "Centred", "Pinned apps and Recommended, above the centred dock"), ("tiles", "Tiles", "App list and live tiles in the corner of a full-width bar"), ("start", "Start screen", "Full-screen live tiles; easiest with a controller"), ("classic", "Classic", "Two columns with your places; glassy, from the corner")] {
            let on = am == v || (am.is_empty() && v == "hearth");
            c = c.push(irow(&p, Ic::Menu, l, d, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", ui::Kind::Normal, false, Some(Msg::Set(json!({"appMenu": v})))) }));
        }
    }
    c = c.push(head(&p, "Menus"));
    let ms = sh.d_str("menuStyle");
    let ms = if ms.is_empty() { "today".to_string() } else { ms };
    c = c.push(irow(&p, Ic::Menu, "Right-click menu", if plasma { "Hearth's menus on Mantel (Debian's desktop): KDE Plasma draws its own here, in the look's colours" } else { "How the menu looks on files, the desktop, the bar, the dock and the app menus" }, seg(&p, [("Today", "today"), ("Windows 11 style", "windows"), ("Compact list", "compact"), ("Tiles and groups", "tiles")].iter().map(|(l, v)| (l.to_string(), ms == *v, Some(Msg::Set(json!({"menuStyle": v}))))).collect())));
    c.into()
}
fn titlecase(s: &str) -> String {
    let mut ch = s.chars();
    ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
}

// ---------------------------------------------------------------- Wallpaper per screen, slideshow & picture of the day
pub fn wallpaper_more(a: &App) -> E<'_> {
    let sh = &a.sh;
    let st = &a.pers;
    let p = sh.p;
    let plasma = kit::on_plasma();
    let mut c = kit::Col::new(st.skip);
    if !st.msg.is_empty() {
        c = c.push(kit::said(&p, &st.msg, st.msg_bad));
    }
    let own = kit::obj(sh, "wallScreens");
    let show = Value::Object(kit::obj(sh, "wallShow"));
    let potd = Value::Object(kit::obj(sh, "wallPotd"));
    let pair = Value::Object(kit::obj(sh, "wallPair"));
    c = c.push(head(&p, "Your screens"));
    let scr = screens(st);
    if plasma && st.walls.is_none() {
        c = c.push(small(&p, if st.walls_err.is_empty() { "Asking KDE Plasma…".to_string() } else { format!("{}.", st.walls_err) }));
    }
    if !plasma {
        c = c.push(small(&p, "Each screen's picture is set where the desktop shows it: on Mantel, its own Settings; here, with KDE Plasma."));
    }
    let mut r = Row::new().spacing(14.0);
    for (i, s) in scr.iter().enumerate() {
        let mine = own.get(&s.name).and_then(Value::as_str).unwrap_or("").to_string();
        let shows = if !mine.is_empty() { kit::base(&mine) } else if kit::b_of(&potd, "on") { "Picture of the day".into() } else if kit::b_of(&show, "on") { "Slideshow".into() } else { "Same as the others".into() };
        let ratio = if s.w > 0 { (s.h as f32 / s.w as f32).clamp(0.4, 0.8) } else { 0.5625 };
        let tw = sz(310.0);
        let pic_name = if !mine.is_empty() { mine.clone() } else { s.image.clone() };
        let thumb = kit::swatch(kit::name_colour(&p, &pic_name), p.accent, iced::Color::WHITE, tw, tw * ratio, None);
        let mut btns = Row::new().spacing(8.0).push(btn(&p, "Choose a picture…", ui::Kind::Normal, false, Some(Msg::Choose(For::Screen(i)))));
        if !mine.is_empty() {
            btns = btns.push(btn(&p, "Same as others", ui::Kind::Normal, false, Some(Msg::ScreenSame(i))));
        }
        r = r.push(container(column![thumb, ts(format!("{}{} · {}×{}", if i == 0 { "Main · " } else { "" }, s.name, s.w, s.h), sz(14.0), p.text), t1(ui::fit(&shows, 40), sz(12.0), p.muted), btns].spacing(8.0)).padding(14.0).width(Length::Fixed(tw + 28.0)).style(ui::boxed(p.panel, p.r(12.0), None)));
        if i % 2 == 1 {
            c = c.push(r);
            r = Row::new().spacing(14.0);
        }
    }
    c = c.push(r);
    c = c.push(small(&p, "A screen without its own picture shows the slideshow when it's on, or the wallpaper from Personalization."));

    c = c.push(head(&p, "Slideshow"));
    let folder = kit::s_of(&show, "folder");
    let son = kit::b_of(&show, "on");
    c = c.push(irow(&p, Ic::Image, "Slideshow", if folder.is_empty() { "Pick a folder of pictures first".to_string() } else { format!("Pictures from {folder}") }, switch(&p, son, Some(Msg::Show("on", json!(!son))))));
    c = c.push(iact(&p, Ic::G(G::Folder), if folder.is_empty() { "Pick a folder" } else { "Change the folder" }, if folder.is_empty() { "JPEG, PNG and WebP pictures in it".to_string() } else { folder.clone() }, false, Some(Msg::Choose(For::Folder))));
    let mins = kit::n_of(&show, "minutes", 30.0) as i64;
    c = c.push(irow(&p, Ic::G(G::Clock), "Change the picture every", "", seg(&p, [("5 min", 5i64), ("15 min", 15), ("30 min", 30), ("Hour", 60), ("Day", 1440)].iter().map(|(l, v)| (l.to_string(), mins == *v, Some(Msg::Show("minutes", json!(v))))).collect())));
    let shuf = kit::b_of(&show, "shuffle");
    c = c.push(irow(&p, Ic::Sync, "Shuffle", "In a random order instead of by name", switch(&p, shuf, Some(Msg::Show("shuffle", json!(!shuf))))));
    if plasma {
        c = c.push(small(&p, "KDE Plasma's slideshow: right-click the desktop › Next Wallpaper Image for the next one now."));
    }

    c = c.push(head(&p, "Picture of the day"));
    let pon = kit::b_of(&potd, "on");
    let has = !plasma || has_plugin(st, "org.kde.potd");
    let src = { let x = kit::s_of(&potd, "source"); if x.is_empty() { "bing".to_string() } else { x } };
    let src_name = match src.as_str() { "apod" => "NASA's Astronomy Picture of the Day", "wikimedia" => "Wikimedia Commons", _ => "Bing" };
    let psub = if !has { "Needs KDE's picture of the day (the kdeplasma-addons package), which isn't installed".to_string() } else if !pon { "A new wallpaper every day from the internet. Off until you turn it on".to_string() } else { format!("Today's picture from {src_name}: downloaded once a day and kept, so it works offline too") };
    c = c.push(kit::irow_if(&p, has, Ic::Image, "Picture of the day", psub, switch(&p, pon, if has { Some(Msg::Potd("on", json!(!pon))) } else { None })));
    c = c.push(irow(&p, Ic::Globe, "From", "", seg(&p, [("Bing", "bing"), ("NASA APOD", "apod"), ("Wikimedia", "wikimedia")].iter().map(|(l, v)| (l.to_string(), src == *v, Some(Msg::Potd("source", json!(v))))).collect())));
    if pon {
        c = c.push(iact(&p, Ic::Sync, "Look again now", "Downloaded once a day and kept for a week, so it works offline too", false, Some(Msg::PotdAgain)));
    }

    c = c.push(head(&p, "Light and dark"));
    let paon = kit::b_of(&pair, "on");
    let dark = sh.d_str("mode") != "light";
    c = c.push(irow(&p, Ic::G(G::Moon), "A wallpaper for light and one for dark", if paon { format!("Follows the look: now {}", if dark { "dark" } else { "light" }) } else { "Changes with the look's light and dark mode (and Sunset)".to_string() }, switch(&p, paon, Some(Msg::Pair("on", json!(!paon))))));
    let mut pr = Row::new().spacing(14.0);
    for (k, title) in [("light", "Light"), ("dark", "Dark")] {
        let f = kit::s_of(&pair, k);
        let now = (k == "dark") == dark;
        let bg = hearth_ui::theme::hex(if k == "dark" { "#14110f" } else { "#ece7e1" }).unwrap();
        let pic: E<'_> = if f.is_empty() {
            container(t1("The wallpaper as it is", sz(12.0), hearth_ui::theme::hex(if k == "dark" { "#a89f96" } else { "#6b625a" }).unwrap())).center_x(Length::Fixed(sz(272.0))).center_y(Length::Fixed(sz(140.0))).style(ui::boxed(bg, 8.0, Some((Pal::a(p.text, 0.12), 1.0)))).into()
        } else {
            kit::swatch(kit::name_colour(&p, &f), p.accent, iced::Color::WHITE, sz(272.0), sz(140.0), None)
        };
        let mut btns = Row::new().spacing(8.0).push(btn(&p, "Choose…", ui::Kind::Normal, false, Some(Msg::Choose(For::Pair(k)))));
        if !f.is_empty() {
            btns = btns.push(btn(&p, "Clear", ui::Kind::Normal, false, Some(Msg::Pair(k, json!("")))));
        }
        pr = pr.push(container(column![pic, ts(if f.is_empty() { title.to_string() } else { format!("{title} · {}", ui::fit(&kit::base(&f), 30)) }, sz(14.0), p.text), btns].spacing(8.0)).padding(14.0).width(Length::Fixed(sz(300.0))).style(ui::boxed(p.panel, p.r(12.0), Some((if paon && now { p.accent } else { Pal::a(p.text, 0.07) }, if paon && now { 2.0 } else { 1.0 })))));
    }
    c = c.push(pr);
    c.into()
}

pub fn dialog<'a>(_a: &'a App, _id: &str) -> Option<E<'a>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screens_get_their_names() {
        let mut st = State::default();
        st.walls = Some(json!({"desktops": [{"screen": 1, "x": 1920, "y": 0, "width": 1280, "height": 1024, "plugin": "org.kde.image", "image": "file:///a/b.jpg"}, {"screen": 0, "x": 0, "y": 0, "width": 1920, "height": 1080, "plugin": "org.kde.image", "image": ""}],
                               "ks": {"outputs": [{"name": "eDP-1", "pos": {"x": 0, "y": 0}, "enabled": true}, {"name": "HDMI-A-1", "pos": {"x": 1920, "y": 0}, "enabled": true}]}}));
        let s = screens(&st);
        assert_eq!(s[0].name, "eDP-1");
        assert_eq!(s[1].name, "HDMI-A-1");
        assert_eq!(s[1].image, "/a/b.jpg");
    }
}
