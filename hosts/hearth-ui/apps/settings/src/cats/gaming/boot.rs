//! Gaming › Startup animation (Mantel's SetBoot.qml, its "animation" part, with SetBootCard.qml and
//! SetBootPreview.qml): what Steam Game Mode plays while it starts. Steam's own, or one of Hearth's; a card opens its
//! preview and only "Use this" (or "Use with its boot splash", which also sets the matching boot splash through
//! hearth-admin) chooses. Through /usr/libexec/nimbus/hearth-boot-look; the animation is yours (copied into Steam's
//! override folder as Steam Game Mode starts).

use crate::cats::apps::kit::{self, *};
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
    st.preview.take().is_some()
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
            } else {
                st.say.bad(r.err)
            }
        }
        M::Preview(v) => st.preview = Some(v),
        M::Close => st.preview = None,
        M::Use(id, label) => {
            st.preview = None;
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

/// A choice's card: its picture's place (the preview sheet's frames aren't drawn here: a display mark stands in), its
/// name, what it looks like, the matching splash, In use, Preview ›.
fn boot_card<'a>(p: &hearth_ui::Pal, item: &Value, in_use: bool, pair: String, msg: M) -> E<'a, M> {
    let pic = container(glyph(G::Display, p.muted, 22.0, 1.8)).center_x(Length::Fixed(128.0)).center_y(Length::Fixed(72.0)).style(ui::boxed(hearth_ui::theme::hex("#0b1016").unwrap_or(p.bg), 0.0, Some((p.line, 1.0))));
    let mut info = column![tm(gs(item, "label"), sz(15.0), p.text), t(gs(item, "about"), sz(12.0), p.muted)].spacing(3.0).width(Length::Fill);
    if !pair.is_empty() {
        info = info.push(t(pair, sz(11.0), p.muted));
    }
    let mut rw = row![pic, info].spacing(14.0).align_y(iced::Center);
    if in_use {
        rw = rw.push(tm("In use", sz(13.0), p.flame));
    }
    rw = rw.push(t("Preview  ›", sz(13.0), p.muted));
    mouse_area(container(rw).padding(Padding { top: 12.0, right: 16.0, bottom: 12.0, left: 12.0 }).width(Length::Fill).style(ui::boxed(p.panel, p.r(10.0), None))).on_press(msg).interaction(mouse::Interaction::Pointer).into()
}

pub fn view<'a>(sh: &'a Shared, st: &'a St) -> E<'a, M> {
    let p = sh.p;
    let s = &st.st;
    let movie = go(s, "movie");
    let cur = gs(movie, "current");
    let splashes = ga(go(s, "splash"), "presets");
    let mut v: Vec<E<M>> = vec![head(&p, if st.saving { "Startup animation (Steam Game Mode) · saving…" } else { "Startup animation (Steam Game Mode)" })];
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
        v.push(boot_card(&p, m, cur == gs(m, "id"), pl, M::Preview(m.clone())));
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
        container(glyph(G::Play, p.muted, 34.0, 1.8)).center_x(Length::Fill).center_y(Length::Fixed(200.0)).style(ui::boxed(hearth_ui::theme::hex("#0b1016").unwrap_or(p.bg), p.r(8.0), None)),
        t(gs(m, "about"), sz(14.0), p.text),
        t(if in_use { "This is the one in use." } else { "Only a preview: nothing changes until you choose Use this." }, sz(12.0), if in_use { p.flame } else { p.muted }),
        buttons(bs),
    ]
    .spacing(14.0);
    Some((ui::dialog_box(&p, 620.0, body), Some(M::Close)))
}
