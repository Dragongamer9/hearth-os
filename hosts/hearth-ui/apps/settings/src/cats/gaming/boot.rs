//! Gaming › Startup animation (Mantel's SetBoot.qml, its "animation" part, with SetBootCard.qml and
//! SetBootPreview.qml): what Steam Game Mode plays while it starts. Steam's own, or one of Hearth's; a card opens its
//! preview and only "Use this" (or "Use with its boot splash", which also sets the matching boot splash through
//! hearth-admin) chooses. Through /usr/libexec/nimbus/hearth-boot-look; the animation is yours (copied into Steam's
//! override folder as Steam Game Mode starts). Each card shows its animation's picture (the preview sheet's poster
//! frame) and plays it while the mouse or the controller's focus is on it; the preview plays it (bootpics.rs).

use crate::bootpics::{self, Flip, Sheet};
use crate::cats::apps::kit::{self, *};
use crate::cats::apps::kit::button;
use crate::shared::Shared;
use crate::widgets::*;
use hearth_ui::ui::{self, glyph, t, tb, tm, G};
use iced::widget::{column, container, mouse_area, row};
use iced::{mouse, Length, Padding, Task};
use serde_json::Value;

#[derive(Default)]
pub struct St {
    pub st: Value,
    pub want: String,
    pub preview: Option<Value>,
    pub saving: bool,
    pub say: Say,
    pub flip: Flip,
}

#[derive(Debug, Clone)]
pub enum M {
    Got(Res),
    Preview(Value),
    Close,
    Use(String, String),
    UseSet(String, String, String),
    Splashed(String, String, Res),
    Movied(String, Res),
    Play(String),
    /// The pictures (poster frames or a sheet's frames) were read: draw again.
    Pics,
    /// The clock while a picture plays.
    Tick,
    Hover(String),
    Unhover(String),
}

fn look(args: &[&str]) -> Vec<String> {
    py(&libexec("hearth-boot-look"), args, 60)
}
fn load() -> Task<M> {
    read(look(&["status"]), M::Got)
}

pub fn open(_sh: &mut Shared, st: &mut St) -> Task<M> {
    st.say.clear();
    st.preview = None;
    // `startup-animation:preview-<id>` opens that one's preview once the list is read.
    st.want = std::mem::take(&mut _sh.part).strip_prefix("preview-").unwrap_or("").to_string();
    load()
}

pub fn back(st: &mut St) -> bool {
    let had = st.preview.take().is_some();
    if had {
        st.flip.halt();
    }
    had
}

/// Each animation's card: its name and its sheet.
fn cards(st: &St) -> Vec<(String, Sheet)> {
    ga(go(&st.st, "movie"), "list").iter().filter_map(|m| bootpics::sheet(m).map(|s| (gs(m, "label"), s))).collect()
}

/// The sheet that plays now (the preview's, the card under the mouse or the focus), if any.
pub fn active(st: &St) -> Option<Sheet> {
    let pv = st.preview.as_ref().and_then(bootpics::sheet);
    bootpics::active(pv, &st.flip, &cards(st))
}

/// Something plays: the clock runs (gaming.rs subscription), at the sheets' frames a second.
pub fn live(st: &St) -> Option<std::time::Duration> {
    active(st).map(|_| bootpics::every(bootpics::geo(&st.st)))
}

fn posters(st: &St) -> Task<M> {
    let Some(g) = bootpics::geo(&st.st) else { return Task::none() };
    match bootpics::load_posters(cards(st).into_iter().map(|c| c.1).collect(), g) {
        Some(f) => Task::perform(f, |_| M::Pics),
        None => Task::none(),
    }
}

fn movie_said(label: &str, r: &Res) -> String {
    format!("{label}{}", if gb(&r.v, "later") { " plays once Steam has been set up." } else { " plays the next time Steam Game Mode starts." })
}

pub fn update(sh: &mut Shared, st: &mut St, m: M) -> Task<M> {
    match m {
        M::Got(r) => {
            if r.ok {
                st.st = r.v;
                let want = std::mem::take(&mut st.want);
                if !want.is_empty() {
                    st.preview = ga(go(&st.st, "movie"), "list").iter().find(|m| gs(m, "id") == want).cloned();
                }
                return posters(st);
            } else {
                st.say.bad(r.err)
            }
        }
        M::Preview(v) => st.preview = Some(v),
        M::Close => {
            st.preview = None;
            st.flip.halt();
        }
        M::Pics => {}
        M::Tick => {
            let a = active(st);
            let g = bootpics::geo(&st.st);
            return match st.flip.tick(a.as_ref(), g) {
                Some(f) => Task::perform(f, |_| M::Pics),
                None => Task::none(),
            };
        }
        M::Hover(path) => st.flip.hover(path),
        M::Unhover(path) => {
            st.flip.unhover(&path);
            if active(st).is_none() {
                st.flip.halt();
            }
        }
        M::Use(id, label) => {
            st.preview = None;
            st.flip.halt();
            if gs(go(&st.st, "movie"), "current") == id {
                return Task::none();
            }
            st.say.clear();
            st.saving = true;
            sh.reread_history = true;
            return read(look(&["movie", &id]), move |r| M::Movied(movie_said(&label, &r), r));
        }
        M::UseSet(splash, movie, label) => {
            st.preview = None;
            st.flip.halt();
            st.say.clear();
            st.saving = true;
            let cur = gs(go(&st.st, "splash"), "current");
            if cur == splash {
                return update(sh, st, M::Splashed(movie, label, Res { ok: true, ..Default::default() }));
            }
            return read(admin(&["boot-splash", &splash], 120), move |r| M::Splashed(movie.clone(), label.clone(), r));
        }
        M::Splashed(movie, label, r) => {
            if !r.ok {
                st.saving = false;
                st.say.bad(r.err);
                return load();
            }
            if gs(go(&st.st, "movie"), "current") == movie {
                st.saving = false;
                st.say.ok(format!("{label}: the splash and the startup animation are in use."));
                return load();
            }
            return read(look(&["movie", &movie]), move |r| {
                let said = format!("{label}: the splash shows from the next start, and the startup animation {}", if gb(&r.v, "later") { "plays once Steam has been set up." } else { "plays the next time Steam Game Mode starts." });
                M::Movied(said, r)
            });
        }
        M::Movied(said, r) => {
            st.saving = false;
            if r.ok {
                st.say.ok(said)
            } else {
                st.say.bad(r.err)
            }
            return load();
        }
        M::Play(file) => {
            if !file.is_empty() {
                kit::open(&["xdg-open", &file]);
            }
        }
    }
    Task::none()
}

fn label_of(list: &[Value], id: &str) -> String {
    list.iter().find(|x| gs(x, "id") == id).map(|x| gs(x, "label")).unwrap_or_default()
}

/// A choice's card (SetBootCard.qml): its picture (the poster frame; it plays while the mouse or the focus is on it),
/// its name, what it looks like, the matching one, In use, Preview ›. Shared with System › Boot splash.
pub fn boot_card<'a, M: Clone + Send + 'static>(p: &hearth_ui::Pal, flip: &Flip, item: &Value, in_use: bool, pair: String, msg: M, hover: impl Fn(String) -> M, unhover: impl Fn(String) -> M) -> E<'a, M> {
    let label = gs(item, "label");
    let about = gs(item, "about");
    crate::nav::control(crate::nav::Kind::Open, vec![Box::new(msg.clone()) as crate::nav::Act], Vec::new(), Vec::new(), String::new(), String::new());
    let nr = crate::nav::item(&label, &about, false);
    let sheet = bootpics::sheet(item);
    let pic = bootpics::picture(p, flip, sheet.as_ref(), bootpics::CARD_W, bootpics::CARD_H, "");
    let mut info = column![tm(label, sz(15.0), p.text), t(about, sz(12.0), p.muted)].spacing(3.0).width(Length::Fill);
    if !pair.is_empty() {
        info = info.push(t(pair, sz(11.0), p.muted));
    }
    let mut rw = row![pic, info].spacing(14.0).align_y(iced::Center);
    if in_use {
        rw = rw.push(tm("In use", sz(13.0), p.flame));
    }
    rw = rw.push(t("Preview  ›", sz(13.0), if nr.lit { p.text } else { p.muted }));
    let c = container(rw).id(nr.key).padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 12.0 }).width(Length::Fill).style(row_style(*p, p.panel, p.r(10.0), nr.lit));
    let mut m = mouse_area(c).on_press(msg).interaction(mouse::Interaction::Pointer);
    if let Some(s) = sheet {
        m = m.on_enter(hover(s.path.clone())).on_exit(unhover(s.path));
    }
    m.into()
}

/// The preview's screen (SetBootPreview.qml): 16:9, the sheet playing, or "No preview for this one in this version".
pub fn preview_screen<'a, M: 'a>(p: &hearth_ui::Pal, flip: &Flip, item: &Value, width: f32) -> E<'a, M> {
    let w = width.min(720.0);
    bootpics::picture(p, flip, bootpics::sheet(item).as_ref(), w, (w * 9.0 / 16.0).round(), "No preview for this one in this version")
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let movie = go(s, "movie");
    let cur = gs(movie, "current");
    let splashes = ga(go(s, "splash"), "presets");
    let mut v = crate::secs::SecVec::new();
    v.push(head(&p, if st.saving { "Startup animation (Steam Game Mode) · saving…" } else { "Startup animation (Steam Game Mode)" }));
    let mut intro = "What Steam Game Mode plays when it starts. Pick one to see it play; it changes only when you choose Use this. Hearth's animations are also in Steam's own Settings › Customization › Startup Movie.".to_string();
    if !movie.is_null() && !gb(movie, "steamReady") {
        intro.push_str(" Steam hasn't been set up yet: your choice is used once it has.");
    }
    v.push(small(&p, intro));
    v.push(r(G::Play, "Steam's own", "The animation Steam picks (or the one you chose in Steam). Choose to use it.").value(if cur == "steam" { "In use" } else { "" }).on(M::Use("steam".into(), "Steam's own animation".into())).view(&p));
    let list = ga(movie, "list");
    for m in list {
        let pair = gs(m, "pair");
        let pl = if pair.is_empty() { String::new() } else { format!("Goes with the {} boot splash", label_of(splashes, &pair)) };
        v.push(boot_card(&p, &st.flip, m, cur == gs(m, "id"), pl, M::Preview(m.clone()), M::Hover, M::Unhover));
    }
    if s.is_null() && st.say.text.is_empty() {
        v.push(words(&p, "Looking…", false));
    } else if list.is_empty() && !s.is_null() {
        v.push(words(&p, "This version of Hearth OS has no startup animations of its own: Steam's own plays.", false));
    }
    opt(&mut v, say_view(&p, &st.say));
    page(v)
}

/// The preview (SetBootPreview.qml): looking changes nothing; only Use this chooses.
pub fn dialog<'a>(sh: &'a Shared, st: &'a St) -> Option<(E<'a, M>, Option<M>)> {
    let p = sh.p;
    let m = st.preview.as_ref()?;
    let id = gs(m, "id");
    let label = gs(m, "label");
    let in_use = gs(go(&st.st, "movie"), "current") == id;
    let pair = gs(m, "pair");
    let can_pair = !pair.is_empty() && gb(go(&st.st, "splash"), "canChange");
    let file = gs(m, "file");
    let mut bs = vec![button(&p, "Use this", true, if in_use { None } else { Some(M::Use(id.clone(), label.clone())) })];
    if can_pair {
        bs.push(button(&p, "Use with its boot splash", false, Some(M::UseSet(pair.clone(), id.clone(), label.clone()))));
    }
    if !file.is_empty() {
        bs.push(button(&p, "Play the video", false, Some(M::Play(file))));
    }
    bs.push(button(&p, "Close", false, Some(M::Close)));
    let body = column![
        tb(format!("Startup animation: {label}"), sz(20.0), p.text),
        container(preview_screen(&p, &st.flip, m, 640.0)).center_x(Length::Fill),
        t(format!("{}. Steam Game Mode plays it once as it starts.", gs(m, "about").trim_end_matches('.')), sz(13.0), p.muted),
        t(if in_use { "This is the one in use." } else { "Only a preview: nothing changes until you choose Use this." }, sz(12.0), if in_use { p.flame } else { p.muted }),
        buttons(bs),
    ]
    .spacing(14.0);
    Some((ui::dialog_box(&p, 680.0, body), Some(M::Close)))
}
