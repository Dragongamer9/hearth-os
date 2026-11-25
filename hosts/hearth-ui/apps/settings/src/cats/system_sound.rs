//! System › Sound and its pages (Mantel's SetSound.qml, SetSoundSpeakers.qml, SetSoundApps.qml, SetPushToTalk.qml,
//! SetSounds.qml). PipeWire through its own tools (wpctl, pactl) and Mantel's hearth-sound (each sound card's
//! profile, the microphone test, balance, a test tone from each speaker, moving an app to other speakers or another
//! microphone: WirePlumber remembers those itself, Hearth keeps no file). Over-amplification is buttons.json's
//! (overamp), with Plasma's own (plasmaparc [General] RaiseMaximumVolume) set too, so Plasma's volume goes to 150%
//! as well. Push to talk is desktop.json's pttOn / pttKey. The sound set is sounds.json's, with KDE's sound theme
//! (kdeglobals [Sounds]) set to the same set on Plasma.

use super::*;
use iced::widget::{column, row};
use hearth_ui::ui::{self, glyph, ts, Kind};
use crate::widgets::btn;
use iced::widget::{container, mouse_area, Row, Space};
use serde_json::json;

fn snd(st: &State) -> data::Sound {
    data::sound(st.raw("snd"))
}

fn overamp(st: &State) -> bool {
    vb(st.file("buttons.json"), "overamp") || vs(st.j("k-sound"), "overamp") == "true"
}

/// One app playing (or recording): its name, its streams (pactl index, PipeWire node id), the device it's on (the
/// sink's or source's index) and its volume (0..1.5).
struct Stream {
    app: String,
    idx: Vec<i64>,
    ids: Vec<String>,
    dev: i64,
    vol: f64,
    muted: bool,
}

fn pct(v: &Value) -> f64 {
    // pactl's "volume": {"front-left": {"value_percent": "45%"}, ...}: the loudest channel.
    v.as_object().map(|m| m.values().map(|c| vs(c, "value_percent").trim_end_matches('%').parse::<f64>().unwrap_or(0.0)).fold(0.0, f64::max)).unwrap_or(0.0) / 100.0
}

fn streams(st: &State, which: &str) -> Vec<Stream> {
    let sec = data::sections(st.raw("streams"));
    let list: Value = serde_json::from_str(sec.get(which).map(|s| s.trim()).unwrap_or("[]")).unwrap_or(Value::Null);
    let mut out: Vec<Stream> = Vec::new();
    for x in list.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let props = at(x, "properties");
        let mut app = kg(props, "application.name");
        if app.is_empty() {
            app = kg(props, "media.name");
        }
        // Monitors and Hearth's own level meters aren't apps.
        if app.is_empty() || kg(props, "stream.monitor") == "true" || app.starts_with("PulseAudio Volume Control") {
            continue;
        }
        let idx = vf(x, "index", -1.0) as i64;
        let id = kg(props, "object.id");
        let dev = vf(x, if which == "play" { "sink" } else { "source" }, -1.0) as i64;
        let vol = pct(at(x, "volume"));
        match out.iter_mut().find(|s| s.app == app) {
            Some(s) => {
                s.idx.push(idx);
                s.ids.push(id);
                s.vol = s.vol.max(vol);
            }
            None => out.push(Stream { app, idx: vec![idx], ids: vec![id], dev, vol, muted: vb(x, "mute") }),
        }
    }
    out.sort_by(|a, b| a.app.to_lowercase().cmp(&b.app.to_lowercase()));
    out
}

/// The devices (sinks or sources): index, name, description, PipeWire serial and id.
fn devices(st: &State, which: &str) -> Vec<(i64, String, String, String, String)> {
    let sec = data::sections(st.raw("streams"));
    let list: Value = serde_json::from_str(sec.get(which).map(|s| s.trim()).unwrap_or("[]")).unwrap_or(Value::Null);
    list.as_array()
        .map(|a| {
            a.iter()
                .filter(|x| !vs(x, "name").ends_with(".monitor"))
                .map(|x| {
                    let props = at(x, "properties");
                    let desc = if vs(x, "description").is_empty() { vs(x, "name") } else { vs(x, "description") };
                    (vf(x, "index", -1.0) as i64, vs(x, "name"), desc, kg(props, "object.serial"), kg(props, "object.id"))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn slider_save(_sh: &mut Shared, st: &mut State, k: &'static str) -> Task<Msg> {
    match k {
        "vol" => {
            let v = st.num.remove("vol").unwrap_or(0.0);
            exec(Ex::new(sv(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "0"])).and(sv(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{v:.2}")])).re("snd").fail("Couldn't change the volume"))
        }
        "mic" => {
            let v = st.num.remove("mic").unwrap_or(0.0);
            exec(Ex::new(sv(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SOURCE@", &format!("{v:.2}")])).re("snd").fail("Couldn't change the microphone's volume"))
        }
        "balance" => {
            let v = st.num.remove("balance").unwrap_or(0.0).round() as i64;
            let sink = cur_out(st).map(|o| vs(o, "name")).unwrap_or_default();
            exec(Ex::new(py_s("hearth-sound", vec!["balance".into(), sink, v.to_string()])).re("outputs").fail("Couldn't change the balance"))
        }
        "appvol" | "recvol" => {
            let who = st.txt("numof").to_string();
            let Some(v) = st.numof.get(&who).copied() else { return Task::none() };
            let Some(rest) = who.split_once(':').map(|x| x.1.to_string()) else { return Task::none() };
            let what = if k == "appvol" { "set-sink-input-volume" } else { "set-source-output-volume" };
            let cmds: Vec<Vec<String>> = rest.split(',').filter(|i| !i.is_empty()).map(|i| sv(&["pactl", what, i, &format!("{}%", (v * 100.0).round() as i64)])).collect();
            exec(Ex::none().and_all(cmds).re("streams").fail("Couldn't change that app's volume"))
        }
        _ => Task::none(),
    }
}

pub fn view<'a>(a: &'a App, page: &'static str) -> El<'a> {
    match page {
        "sound" => sound(a),
        "sound-speakers" => speakers(a),
        "sound-apps" => apps(a),
        "push-to-talk" => ptt(a),
        _ => sounds(a),
    }
}

fn sound(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let s = snd(st);
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    if !s.ok {
        return note(&p, if st.read("snd") { "Sound couldn't be read (PipeWire's wpctl didn't answer)." } else { "Reading sound…" }, st.read("snd"));
    }
    let max = if overamp(st) { 1.5 } else { 1.0 };
    let vol = st.num.get("vol").copied().unwrap_or(s.vol).min(max);
    c = c.push(head(&p, "Volume"));
    c = c.push(srow(&p, G::Sound, "Volume", if s.muted { "Muted".to_string() } else { format!("{}%", (vol * 100.0).round()) }, slider_f32(vol, 0.0..=max, 0.01, |v| Msg::Num("vol", v), Msg::NumDone("vol"))));
    c = c.push(srow(&p, G::Sound, "Mute", "", sw(&p, s.muted, Msg::Exec(Ex::new(sv(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])).re("snd").fail("Couldn't mute")))));
    // Over-amplification asks first (KDE's warning): turning it on shows what it can do, and Turn on.
    let oa = overamp(st);
    let asking = st.on("askoveramp") && !oa;
    let oa_set = |on: bool| -> Msg {
        let mut ex = Ex::none().re("k-sound");
        if plasma() {
            ex = ex.and(kw("plasmaparc", &["General"], "RaiseMaximumVolume", if on { "true" } else { "false" }));
        }
        if !on && s.vol > 1.0 {
            ex = ex.and(sv(&["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "1.0"])).re("snd");
        }
        Msg::Json("buttons.json", json!({"overamp": on}), ex)
    };
    c = c.push(srow(&p, G::Sound, "Over-amplification", "Lets the volume go up to 150%. Sound can distort, and loud sound can damage speakers or your hearing.", sw(&p, oa || asking, if oa { oa_set(false) } else { Msg::Flag("askoveramp", !asking) })));
    if asking {
        c = c.push(card(&p, column![
            ts("Louder than 100% can hurt", sz(15.0), p.warn),
            t("Above 100% sound is made louder by the PC, not the speakers: it can crackle, damage small speakers, and loud sound in headphones can harm your hearing. Turn the volume down before going over 100%.", sz(13.0), p.muted),
            row![btn(&p, "Turn on", Kind::Primary, false, Some(oa_set(true))), btn(&p, "Cancel", Kind::Normal, false, Some(Msg::Flag("askoveramp", false)))].spacing(8.0)
        ].spacing(8.0)));
    }
    if oa && s.vol > 1.001 {
        c = c.push(note(&p, "The volume is over 100%: sound may distort, and loud sound in headphones can harm your hearing.", true));
    }
    c = c.push(link_row(&p, G::Sound, "Volume mixer", "Each app's own volume, and which speakers or headphones it plays on", Msg::Go("sound-apps")));
    c = c.push(head(&p, "More sound settings"));
    c = c.push(link_row(&p, G::Sound, "Speakers & balance", "Left and right balance, and a test tone from each speaker", Msg::Go("sound-speakers")));
    c = c.push(link_row(&p, G::Sound, "Apps' sound", "Which speakers or headphones each app plays on, and scrolling on the dock", Msg::Go("sound-apps")));
    c = c.push(link_row(&p, G::Sound, "Push to talk", if a.sh.d_bool("pttOn", false) { "On: hold your key to talk" } else { "Keep the microphone muted unless you hold a key" }, Msg::Go("push-to-talk")));
    c = c.push(link_row(&p, G::Sound, "Sounds", "Hearth's own short sounds, the freedesktop set, or none, and which events play one", Msg::Go("sounds")));
    let playing = streams(st, "play");
    if !playing.is_empty() {
        c = c.push(head(&p, "Apps playing sound"));
        for x in playing.iter() {
            let key = format!("app:{}", x.idx.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","));
            let v = st.numof.get(&key).copied().unwrap_or(x.vol as f32);
            let k2 = key.clone();
            c = c.push(srow(&p, G::Sound, x.app.clone(), if x.muted { "Muted".to_string() } else { format!("{}%", (v * 100.0).round()) }, slider_f32(v.min(1.5), 0.0..=1.5, 0.01, move |n| Msg::NumOf(k2.clone(), n), Msg::NumDone("appvol"))));
        }
    }
    c = c.push(head(&p, "Play sound through"));
    for o in &s.outs {
        let on = o.name == s.out;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Playing here" } else { "" }, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Exec(Ex::new(sv(&["pactl", "set-default-sink", &o.name])).re("snd").fail("Couldn't switch the speakers")))) }));
    }
    c = c.push(head(&p, "Microphone"));
    let mic = st.num.get("mic").copied().unwrap_or(s.mic).min(1.0);
    c = c.push(srow(&p, G::Sound, "Input volume", if s.mic_muted { "Muted".to_string() } else { format!("{}%", (mic * 100.0).round()) }, row![
        slider_f32(mic, 0.0..=1.0, 0.01, |v| Msg::Num("mic", v), Msg::NumDone("mic")),
        btn(&p, if s.mic_muted { "Unmute" } else { "Mute" }, Kind::Normal, false, Some(Msg::Exec(Ex::new(sv(&["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])).re("snd").fail("Couldn't mute the microphone"))))
    ].spacing(14.0).align_y(iced::Center).into()));
    for o in &s.ins {
        let on = o.name == s.inp;
        c = c.push(srow(&p, G::Sound, o.desc.clone(), if on { "Recording from here" } else { "" }, if on { t1("In use", sz(13.0), p.accent).into() } else { btn(&p, "Use", Kind::Normal, false, Some(Msg::Exec(Ex::new(sv(&["pactl", "set-default-source", &o.name])).re("snd").fail("Couldn't switch the microphone")))) }));
    }
    // The microphone test: a few seconds recorded and played back, kept in memory only (hearth-sound).
    c = c.push(head(&p, "Test your microphone"));
    let test = st.j("mictest");
    let has_test = st.has("mictest");
    let peak = vf(test, "peak", 0.0);
    let level = vs(test, "level");
    let recording = !st.busy.is_empty();
    let sub = if !has_test {
        "Then play it back to hear how you sound to others".to_string()
    } else {
        match level.as_str() {
            "silent" => "Nothing was heard. Check the microphone is picked above and not muted".into(),
            "quiet" => format!("Very quiet ({}% at the loudest). Turn the input volume up", (peak * 100.0).round()),
            "loud" => "Too loud: it clipped. Turn the input volume down".into(),
            _ => format!("Sounds good ({}% at the loudest)", (peak * 100.0).round()),
        }
    };
    c = c.push(act_row(&p, G::Sound, if recording { "Recording… say something" } else if has_test { "Record again" } else { "Record a few seconds" }, sub, if has_test { "Record again" } else { "Record" }, Some(Msg::Exec(Ex::new(py("hearth-sound", &["mic-test", "4"])).store("mictest").fail("The microphone test didn't work").secs(30)))));
    if has_test {
        let bad = level == "loud" || level == "silent";
        c = c.push(ui::bar(vec![(peak.min(1.0) as f32, if bad { p.danger } else { p.accent })], Pal::mix(p.panel2, p.text, 0.08), 8.0, 4.0));
        c = c.push(act_row(&p, G::Play, "Play it back", "", "Play", Some(Msg::Exec(Ex::new(py("hearth-sound", &["mic-play"])).fail("Couldn't play it back").secs(30)))));
    }
    let cards = va(st.j("cards"), "cards");
    if !cards.is_empty() {
        c = c.push(head(&p, "Sound cards"));
        for (i, cd) in cards.iter().enumerate() {
            let card_name = vs(cd, "name");
            let opts: Vec<(String, String)> = va(cd, "profiles").iter().map(|pr| (vs(pr, "name"), format!("{}{}", vs(pr, "description"), if vb(pr, "available") || !present(pr, "available") { "" } else { " (not plugged in)" }))).collect();
            let g = match vs(cd, "kind").as_str() {
                "hdmi" => G::Display,
                "bluetooth" => G::Bt,
                _ => G::Sound,
            };
            let cn = card_name.clone();
            c = c.push(chooser(a, &format!("card{i}"), g, vs(cd, "description"), "Which outputs and inputs it offers: stereo, surround, HDMI, a headset's microphone", opts, &vs(cd, "active"), move |v| {
                Msg::Exec(Ex::new(py_s("hearth-sound", vec!["profile".into(), cn.clone(), v])).ok("Saved.").re("cards").fail("Couldn't change the sound card's profile"))
            }));
        }
    }
    c.into()
}

fn cur_out(st: &State) -> Option<&Value> {
    let o = va(st.j("outputs"), "outputs");
    let pick = st.txt("spk");
    o.iter().find(|x| vs(x, "name") == pick).or_else(|| o.iter().find(|x| vb(x, "default"))).or_else(|| o.first())
}

/// Speakers & balance: the output you pick, its left/right balance and its speakers around you; press one to hear a
/// short tone from it (hearth-sound balance / speaker-test).
fn speakers(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    if let Some(w) = waiting(a, "outputs", "your speakers") {
        return w;
    }
    let all = va(st.j("outputs"), "outputs");
    c = c.push(head(&p, "Output"));
    if all.is_empty() {
        c = c.push(info_row(&p, G::Info, "No speakers or headphones found", "Plug some in, or check Bluetooth"));
        return c.into();
    }
    let Some(o) = cur_out(st) else { return c.into() };
    if all.len() > 1 {
        let opts: Vec<(String, String)> = all.iter().map(|x| (vs(x, "name"), format!("{}{}", vs(x, "description"), if vb(x, "default") { " (in use)" } else { "" }))).collect();
        c = c.push(chooser(a, "spk", G::Sound, "Speakers or headphones", "Which one these settings are for", opts, &vs(o, "name"), |v| Msg::Text("spk", v)));
    } else {
        c = c.push(info_row(&p, G::Sound, vs(o, "description"), "In use"));
    }
    let chans = va(o, "channels");
    let stereo = chans.iter().any(|x| vs(x, "id").contains("left")) && chans.iter().any(|x| vs(x, "id").contains("right"));
    if stereo {
        c = c.push(head(&p, "Balance"));
        let b = st.num.get("balance").copied().unwrap_or(vf(o, "balance", 0.0) as f32);
        let word = if b.round() == 0.0 { "Centre".to_string() } else if b < 0.0 { format!("{}% left", -b.round()) } else { format!("{}% right", b.round()) };
        c = c.push(srow(&p, G::Sound, "Left and right", format!("{word} · moves the sound towards the left or the right speaker"), slider_f32(b, -100.0..=100.0, 5.0, |v| Msg::Num("balance", v), Msg::NumDone("balance"))));
        if vf(o, "balance", 0.0) != 0.0 {
            c = c.push(act_row(&p, G::Update, "Centre the balance", "", "Centre", Some(Msg::Exec(Ex::new(py_s("hearth-sound", vec!["balance".into(), vs(o, "name"), "0".into()])).re("outputs")))));
        }
    }
    if !chans.is_empty() {
        c = c.push(head(&p, "Test your speakers"));
        let sink = vs(o, "name");
        let tone = |id: &str| Msg::Exec(Ex::new(py_s("hearth-sound", vec!["speaker-test".into(), sink.clone(), id.to_string()])).fail("Couldn't play the tone"));
        // The room: front, sides with you in the middle, rear; each speaker where it stands.
        let rows_of: [&[&str]; 3] = [&["front-left", "front-left-of-center", "front-center", "mono", "lfe", "front-right-of-center", "front-right"], &["side-left", "you", "side-right"], &["rear-left", "rear-center", "rear-right"]];
        let mut room = Column::new().spacing(18.0).align_x(iced::Center);
        for ids in rows_of {
            let mut r = Row::new().spacing(14.0).align_y(iced::Center);
            for id in ids {
                if *id == "you" {
                    r = r.push(column![container(glyph(G::User, p.accent, sz(28.0), 1.8)).center_x(Length::Fixed(74.0)).center_y(Length::Fixed(74.0)).style(ui::boxed(Pal::a(p.accent, 0.14), 37.0, Some((Pal::a(p.accent, 0.5), 1.0)))), t1("You", sz(12.0), p.muted)].spacing(4.0).align_x(iced::Center));
                    continue;
                }
                if let Some(ch) = chans.iter().find(|x| vs(x, "id") == *id) {
                    r = r.push(
                        mouse_area(container(column![glyph(G::Play, p.text, sz(16.0), 1.8), t1(vs(ch, "label"), sz(12.0), p.text)].spacing(3.0).align_x(iced::Center)).center_x(Length::Fixed(132.0)).center_y(Length::Fixed(60.0)).style(ui::boxed(p.panel2, 14.0, Some((p.line, 1.0)))))
                            .on_press(tone(id))
                            .interaction(iced::mouse::Interaction::Pointer),
                    );
                } else if ids.len() == 3 {
                    r = r.push(Space::new().width(Length::Fixed(132.0)));
                }
            }
            room = room.push(r);
        }
        c = c.push(card(&p, container(room).center_x(Length::Fill)));
        // One after another: a tone from each, a moment apart.
        let mut turn = Ex::none().fail("Couldn't play the tones").secs(60);
        for (i, ch) in chans.iter().enumerate() {
            if i > 0 {
                turn = turn.and(sv(&["sleep", "1.5"]));
            }
            turn = turn.and(py_s("hearth-sound", vec!["speaker-test".into(), sink.clone(), vs(ch, "id")]));
        }
        c = c.push(act_row(&p, G::Play, "Play each speaker in turn", "A short tone from every speaker, one after another", "Play", Some(Msg::Exec(turn))));
    }
    c.into()
}

/// Apps' sound: each app playing with its volume and which speakers it plays on, each app recording and which
/// microphone (hearth-sound route: WirePlumber moves it and remembers it for that app), and the dock's scrolling.
fn apps(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    let outs = devices(st, "sinks");
    let ins = devices(st, "sources");
    c = c.push(head(&p, "Apps playing sound"));
    let playing = streams(st, "play");
    if playing.is_empty() {
        c = c.push(info_row(&p, G::Sound, "No app is playing sound right now", "Start a video, a game or music and it shows up here"));
    }
    for (i, x) in playing.iter().enumerate() {
        let key = format!("app:{}", x.idx.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","));
        let v = st.numof.get(&key).copied().unwrap_or(x.vol as f32);
        let k2 = key.clone();
        c = c.push(srow(&p, G::Sound, x.app.clone(), format!("{}%", (v * 100.0).round()), slider_f32(v.min(1.5), 0.0..=1.5, 0.01, move |n| Msg::NumOf(k2.clone(), n), Msg::NumDone("appvol"))));
        if outs.len() > 1 {
            let now = outs.iter().find(|d| d.0 == x.dev).map(|d| d.1.clone()).unwrap_or_default();
            let ids = x.ids.clone();
            let devs = outs.clone();
            c = c.push(chooser(a, &format!("plays{i}"), G::Sound, "Plays on", format!("Send {} to other speakers or headphones; the rest stays where it is", x.app), outs.iter().map(|d| (d.1.clone(), d.2.clone())).collect(), &now, move |v| {
                let d = devs.iter().find(|d| d.1 == v).cloned().unwrap_or_default();
                let mut args = vec!["route".to_string(), d.3.clone(), d.4.clone()];
                args.extend(ids.iter().cloned());
                Msg::Exec(Ex::new(py_s("hearth-sound", args)).re("streams").fail("Couldn't move that app's sound"))
            }));
        }
    }
    c = c.push(head(&p, "Apps using a microphone"));
    let rec = streams(st, "rec");
    if rec.is_empty() {
        c = c.push(info_row(&p, G::Sound, "No app is recording right now", "Join a call or start recording and it shows up here"));
    }
    for (i, x) in rec.iter().enumerate() {
        let now = ins.iter().find(|d| d.0 == x.dev).map(|d| d.1.clone()).unwrap_or_default();
        let ids = x.ids.clone();
        let devs = ins.clone();
        let mut opts: Vec<(String, String)> = vec![(String::new(), "The default microphone".into())];
        opts.extend(ins.iter().map(|d| (d.1.clone(), d.2.clone())));
        let now_desc = ins.iter().find(|d| d.0 == x.dev).map(|d| d.2.clone()).unwrap_or_default();
        c = c.push(chooser(a, &format!("rec{i}"), G::Sound, x.app.clone(), format!("Records from {now_desc}; other apps stay as they are"), opts, &now, move |v| {
            let (serial, id) = devs.iter().find(|d| d.1 == v).map(|d| (d.3.clone(), d.4.clone())).unwrap_or(("-1".into(), "-1".into()));
            let mut args = vec!["route".to_string(), serial, id];
            args.extend(ids.iter().cloned());
            Msg::Exec(Ex::new(py_s("hearth-sound", args)).re("streams").fail("Couldn't move that app's microphone"))
        }));
    }
    if !rec.is_empty() {
        c = c.push(line(&p, "The app keeps this microphone the next time it records, until you pick another. The default microphone is in Sound."));
    }
    c = c.push(head(&p, "The dock"));
    let scroll = a.sh.d_bool("dockScrollVolume", false);
    c = c.push(srow(&p, G::Sound, "Scroll on an app to change its volume", if plasma() { "In Hearth's dock (Mantel): with the mouse over an app's icon, scroll up for louder and down for quieter" } else { "With the mouse over an app's icon in the dock, scroll up for louder and down for quieter" }, sw(&p, scroll, Msg::Desk(json!({"dockScrollVolume": !scroll}), Ex::none()))));
    c = c.push(head(&p, "More"));
    c = c.push(link_row(&p, G::Sound, "Sound", "Volume, speakers and headphones, the microphone", Msg::Go("sound")));
    c.into()
}

const PTT_KEYS: [(&str, &str); 7] = [("f13", "F13"), ("f14", "F14"), ("pause", "Pause"), ("scrolllock", "Scroll Lock"), ("ctrl-grave", "Ctrl+`"), ("meta-alt-space", "Super+Alt+Space"), ("ctrl-alt-t", "Ctrl+Alt+T")];

/// Push to talk: the microphone stays muted and holding your key lets it hear you (desktop.json's pttOn / pttKey;
/// Mantel's PushToTalk.qml runs it through KWin's global shortcuts).
fn ptt(a: &App) -> El<'_> {
    let p = a.sh.p;
    let on = a.sh.d_bool("pttOn", false);
    let key = { let k = a.sh.d_str("pttKey"); if k.is_empty() { "f13".to_string() } else { k } };
    let label = PTT_KEYS.iter().find(|k| k.0 == key).map(|k| k.1.to_string()).unwrap_or_else(|| key.clone());
    // Mantel (Hearth's desktop) runs push to talk; KDE Plasma has none of its own, so on Plasma the switch is kept
    // for Mantel and says so instead of doing nothing.
    let here = std::env::var("HEARTH_WM").map(|w| w == "kwin").unwrap_or(false) && !plasma();
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    if !here {
        c = c.push(card(&p, column![ts("Push to talk needs Mantel on KWin", sz(15.0), p.text), t("It runs in Hearth's own desktop. KDE Plasma has no push to talk of its own, so it's off here; your key is kept for Mantel.", sz(13.0), p.muted)].spacing(4.0)));
    }
    c = c.push(head(&p, "Push to talk"));
    c = c.push(srow(&p, G::Sound, "Push to talk", if on { format!("Your microphone is muted. Hold {label} to talk") } else { "Mute the microphone unless you hold a key".to_string() }, switch(&p, on, if here || on { Some(Msg::Desk(json!({"pttOn": !on}), Ex::none())) } else { None })));
    c = c.push(chooser(a, "pttkey", G::Grid, "Key to hold", "A key games and chats don't use. F13 and F14 are on many gaming keyboards and mice", PTT_KEYS.iter().map(|(i, l)| (i.to_string(), l.to_string())).collect(), &key, |v| Msg::Desk(json!({"pttKey": v}), Ex::none())));
    c = c.push(head(&p, "How it works"));
    let state = |g: G, title: &'static str, sub: &'static str, lit: bool| -> El<'static> {
        container(row![
            container(glyph(g, if lit { p.ink } else { p.muted }, sz(22.0), 1.8)).center_x(Length::Fixed(46.0)).center_y(Length::Fixed(46.0)).style(ui::boxed(if lit { p.accent } else { p.panel3 }, 23.0, None)),
            column![ts(title, sz(15.0), p.text), t(sub, sz(13.0), p.muted)].spacing(2.0)
        ].spacing(14.0).align_y(iced::Center))
        .padding(18.0)
        .width(Length::Fill)
        .style(ui::boxed(if lit { Pal::a(p.accent, 0.13) } else { p.panel }, p.r(10.0), Some((if lit { Pal::a(p.accent, 0.45) } else { p.line }, 1.0))))
        .into()
    };
    c = c.push(row![state(G::Sound, "Key held", "The microphone hears you", true), state(G::Stop, "Key let go", "The microphone is muted", false)].spacing(10.0));
    c = c.push(line(&p, "When push to talk is turned off, the microphone goes back to how it was before."));
    c.into()
}

const EVENTS: [(&str, &str, &str, bool); 11] = [
    ("message-new-instant", "Notification", "For apps whose Sound is on (Settings › Notifications)", true),
    ("dialog-warning", "Warning", "Hearth's own warnings: low space, a settings file put back", true),
    ("dialog-error", "Something went wrong", "Urgent notifications", true),
    ("complete", "Finished", "A copy, download or other job is done", true),
    ("device-added", "Drive plugged in", "A USB stick, card or disk", true),
    ("device-removed", "Drive removed", "", true),
    ("battery-low", "Battery low", "", true),
    ("power-plug", "Charger plugged in", "", false),
    ("power-unplug", "Charger unplugged", "", false),
    ("audio-volume-change", "Volume keys", "A click as the volume changes", false),
    ("bell", "Caps Lock", "As Caps Lock goes on or off", false),
];

/// Sounds: the sound set (Hearth's own, the freedesktop set when it's installed, none) and which events play one
/// (sounds.json, only what you change), each with Play. On Plasma, KDE's sound theme follows the set
/// (kdeglobals [Sounds] Theme / Enable), so Plasma's own sounds are the same.
fn sounds(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let f = st.file("sounds.json");
    let set = match vs(f, "set").as_str() {
        "freedesktop" => "freedesktop",
        "off" => "off",
        _ => "hearth",
    };
    let fdo = st.raw("fdo").trim() == "yes";
    let mut c = crate::widgets::Secs::on(a, 0).spacing(10.0);
    c = c.push(head(&p, "Sound set"));
    let pick = |v: &'static str| -> Msg {
        let mut ex = Ex::none();
        if plasma() {
            ex = ex.and(kw("kdeglobals", &["Sounds"], "Enable", if v == "off" { "false" } else { "true" }));
            if v != "off" {
                ex = ex.and(kw("kdeglobals", &["Sounds"], "Theme", v));
            }
        }
        Msg::Json("sounds.json", json!({"set": v}), ex.re("k-sound"))
    };
    c = c.push(srow(&p, G::Sound, "System sounds", if set == "freedesktop" && !fdo { "The freedesktop sounds aren't installed: Hearth's own play instead" } else { "Short sounds for notifications, warnings and devices" }, seg(&p, vec![
        ("Hearth".into(), set == "hearth", Some(pick("hearth"))),
        ("Freedesktop".into(), set == "freedesktop", if fdo { Some(pick("freedesktop")) } else { None }),
        ("None".into(), set == "off", Some(pick("off"))),
    ])));
    c = c.push(info_row(&p, G::Info, "Where they come from", if set == "freedesktop" { "The freedesktop sound theme (sound-theme-freedesktop, GPL-2+ and CC-BY / CC-BY-SA by their authors), played from where it's installed" } else { "Hearth's own: made for Hearth from simple tones, nothing recorded or borrowed (GPL-3.0, like Hearth OS)" }));
    c = c.push(head(&p, "Events"));
    let evs = at(f, "events");
    let own = run::shell_dir().join("../sounds/hearth/stereo");
    for (id, label, sub, dflt) in EVENTS {
        let on = at(evs, id).as_bool().unwrap_or(dflt);
        let mut all = evs.as_object().cloned().unwrap_or_default();
        all.insert(id.to_string(), Value::Bool(!on));
        let file = if set == "freedesktop" { format!("/usr/share/sounds/freedesktop/stereo/{id}.oga") } else { own.join(format!("{id}.wav")).to_string_lossy().to_string() };
        let fallback = own.join(format!("{id}.wav")).to_string_lossy().to_string();
        let play = Msg::App(sv(&["sh", "-c", "f=\"$1\"; [ -f \"$f\" ] || f=\"$2\"; [ -f \"$f\" ] || exit 0; if command -v pw-play >/dev/null; then exec timeout 10 pw-play --media-role=Notification \"$f\"; elif command -v paplay >/dev/null; then exec timeout 10 paplay \"$f\"; fi", "sh", &file, &fallback]));
        let ctl: El = row![
            pill(&p, "Play", false, if set == "off" { None } else { Some(play) }),
            switch(&p, on, if set == "off" { None } else { Some(Msg::Json("sounds.json", json!({"events": Value::Object(all)}), Ex::none())) })
        ]
        .spacing(16.0)
        .align_y(iced::Center)
        .into();
        c = c.push(srow(&p, G::Sound, label, sub, ctl));
    }
    if set == "off" {
        c = c.push(line(&p, "The sound set is None: no event plays a sound."));
    }
    c.into()
}
