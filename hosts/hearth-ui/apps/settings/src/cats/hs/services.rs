//! Services (SetHomeServices.qml), AI chat (SetHomeAI.qml), AI devices (SetHomeAIDevices.qml), Server apps
//! (SetHomeApps.qml), Phone notifications (SetHomePhone.qml) and Gaming (SetHomeGaming.qml).

use super::kit::*;
use hearth_ui::Pal;
use serde_json::{json, Value};

fn list_shown(st: &St) -> bool {
    let sh = st.get("share");
    b(sh, "server") || b(sh, "settings.hub")
}

pub fn reads(page: &str, st: &St) -> Vec<Rd> {
    match page {
        "hs-services" => vec![rd("sv", H::Hs, &["services"]), rd("status", H::Hs, &["status"])],
        "hs-ai" => vec![rd("ai", H::Hs, &["ai"])],
        "hs-aidevices" => {
            let mut v = vec![rd("share", H::AiShare, &["status"])];
            if list_shown(st) {
                v.push(rd("list", H::AiShare, &["devices"]));
            }
            v
        }
        "hs-apps" => vec![rd("list", H::Apps, &["list"]), rd("worlds", H::Apps, &["worlds"])],
        "hs-phone" => vec![rd("ntfy", H::Apps, &["ntfy"])],
        _ => vec![rd("gaming", H::Hs, &["gaming"])],
    }
}

pub fn follow(_page: &str, slot: &str, st: &St) -> Vec<Rd> {
    if slot == "share" && list_shown(st) && !st.d.contains_key("list") {
        return vec![rd("list", H::AiShare, &["devices"])];
    }
    Vec::new()
}

pub fn live(page: &str, st: &St) -> bool {
    match page {
        "hs-services" => {
            let sv_ = st.get("sv");
            ["media", "ai", "pictures", "voices", "studio"].iter().any(|k| b(sv_, &format!("{k}.installing"))) || b(sv_, "pictures.video.installing")
        }
        "hs-ai" => {
            let ai = st.get("ai");
            b(ai, "ai.installing") || arr(ai, "models").iter().any(|m| b(m, "pulling")) || (b(ai, "custom") && !b(ai, "custom.done") && !b(ai, "custom.error"))
        }
        _ => arr(st.get("list"), "apps").iter().any(|a| b(a, "installing")),
    }
}

/// hearth-home-server's model_name_ok (it checks again): an Ollama name (qwen3:8b, user/model:tag) or a Hugging Face
/// GGUF (hf.co/user/repo:Q4_K_M).
pub fn model_name_ok(n_: &str) -> bool {
    if n_.is_empty() || n_.len() > 200 || n_.contains("..") {
        return false;
    }
    let seg = |x: &str, lower: bool, max: usize| -> bool {
        let mut c = x.chars();
        match c.next() {
            Some(f) if (if lower { f.is_ascii_lowercase() || f.is_ascii_digit() } else { f.is_ascii_alphanumeric() }) => {}
            _ => return false,
        }
        x.len() <= max + 1 && c.all(|ch| (if lower { ch.is_ascii_lowercase() || ch.is_ascii_digit() } else { ch.is_ascii_alphanumeric() }) || "_.-".contains(ch))
    };
    let tag_ok = |t: &str, max: usize| !t.is_empty() && t.len() <= max && t.chars().all(|ch| ch.is_ascii_alphanumeric() || "_.-".contains(ch));
    if let Some(rest) = n_.strip_prefix("hf.co/") {
        let (path, tag) = match rest.split_once(':') {
            Some((a, t)) => (a, Some(t)),
            None => (rest, None),
        };
        let parts: Vec<&str> = path.split('/').collect();
        return parts.len() == 2 && seg(parts[0], false, 95) && seg(parts[1], false, 95) && tag.map(|t| tag_ok(t, 40)).unwrap_or(true);
    }
    let (path, tag) = match n_.split_once(':') {
        Some((a, t)) => (a, Some(t)),
        None => (n_, None),
    };
    let parts: Vec<&str> = path.split('/').collect();
    (parts.len() == 1 || parts.len() == 2) && parts.iter().all(|x| seg(x, true, 60)) && tag.map(|t| tag_ok(t, 60)).unwrap_or(true)
}

pub fn submit(st: &mut St, key: &str, text: String) -> Result<Vec<Act>, String> {
    if let Some(rest) = key.strip_prefix("opt:") {
        let (id, k) = rest.split_once(':').unwrap_or((rest, ""));
        let label = arr(st.get("list"), "apps").iter().find(|a| s(a, "id") == id).and_then(|a| arr(a, "opts").iter().find(|o| s(o, "key") == k).map(|o| s(o, "label"))).unwrap_or_default();
        return Ok(vec![act_v(H::Apps, vec!["opt".into(), id.into(), k.into(), text.trim().into()]).ok(format!("{label} saved ✓"))]);
    }
    if let Some(id) = key.strip_prefix("pw:") {
        return Ok(vec![act_v(H::Apps, vec!["password".into(), id.into()]).input(text).ok("Password saved ✓ (kept only on this PC)")]);
    }
    match key {
        "webpw" => {
            if text.chars().count() < 8 {
                return Err("Use at least 8 characters".into());
            }
            Ok(vec![act(H::Hs, &["web-password"]).input(text).ok("Web password set ✓").set("pwOpen", Value::Null)])
        }
        "vspin" => {
            let pin = text.trim().to_string();
            if pin.len() < 4 || pin.len() > 12 || !pin.chars().all(|c| c.is_ascii_digit()) {
                st.t.insert(key.into(), String::new());
                return Err("Use 4 to 12 digits (numbers only)".into());
            }
            Ok(vec![act(H::Hs, &["studio-pin"]).input(pin).ok("PIN set ✓").set("vsPinOpen", Value::Null)])
        }
        "custom" => {
            let t = text.trim().to_string();
            if !model_name_ok(&t) {
                return Err("That isn't a model name (letters, digits and . : _ / - only)".into());
            }
            Ok(vec![act_v(H::Hs, vec!["ai-pull".into(), t.clone()]).ok(format!("Downloading {t}… (Ollama checks every part)")).set("customOpen", Value::Null).secs(60)])
        }
        "ask" => {
            let m = ask_model(st);
            st.setl("answer", json!("Thinking…"));
            Ok(vec![act_v(H::Hs, vec!["ai-ask".into(), m]).input(text).secs(300).no_reload().set_from("answer", "answer").fail_to("answer")])
        }
        "code" => Ok(vec![act(H::AiShare, &["code-set"]).input(text).ok("Share code set ✓ (only its hash is kept)").set("codeField", Value::Null)]),
        "server" => {
            let t = text.trim().to_string();
            Ok(vec![act_v(H::Apps, vec!["ntfy-set".into(), "server".into(), t]).store("ntfy").ok("Server saved ✓").set("serverOpen", Value::Null)])
        }
        _ => Ok(Vec::new()),
    }
}

fn hs_set(k: &str, v: &str, ok: &str) -> Msg {
    act(H::Hs, &["set", k, v]).ok(ok).msg()
}
fn onoff(b_: bool) -> &'static str {
    if b_ { "on" } else { "off" }
}

fn app_line(a: &Value, label: &str) -> String {
    if !a.is_object() {
        return String::new();
    }
    if b(a, "installing") {
        return format!("Downloading {label}… {}%", n(a, "progress"));
    }
    if b(a, "error") {
        return format!("Didn't install: {}", s(a, "error"));
    }
    if !b(a, "installed") {
        return "Not installed yet".into();
    }
    if !b(a, "on") {
        return format!("Off · {label} {}", s(a, "version"));
    }
    if b(a, "paused") {
        return "Paused while you play".into();
    }
    format!("{} · {label} {}", if b(a, "active") { "Running" } else { "Starting…" }, s(a, "version"))
}

fn run_chip(a: &Value, healthy: bool) -> (String, Ck) {
    if !b(a, "on") {
        return ("Off".into(), Ck::Off);
    }
    if b(a, "paused") {
        return ("Paused".into(), Ck::Off);
    }
    if b(a, "active") && healthy {
        ("Running".into(), Ck::Ok)
    } else {
        ("Starting".into(), Ck::Off)
    }
}

fn listen_line(a: &Value, port: &str, tail_one: &str) -> String {
    let l = arr(a, "listen");
    let mid = if l.len() > 1 { format!(" and {}", vs(&l[1])) } else if l.len() == 1 { tail_one.to_string() } else { " and its Tailscale address".into() };
    format!("Your devices through Tailscale, and this PC: it listens only on this PC{mid}, and Hearth's firewall rule lets only Tailscale reach port {port}")
}

// ------------------------------------------------------------------ Services
pub fn services<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let sv_ = st.get("sv");
    let stv = st.get("status");
    let cfg = at(stv, "settings");
    let ts_only = b(cfg, "tsOnly");
    let media = at(sv_, "media");
    let web = at(sv_, "web");
    let ai = at(sv_, "ai");
    let pic = at(sv_, "pictures");
    let voi = at(sv_, "voices");
    let vs_ = at(sv_, "studio");
    let pic_owner_runs = pic.is_object() && (b(pic, "owner.active") || b(pic, "owner.enabled")) && !b(pic, "takenOver");
    let ai_shown = b(stv, "seen.commandCenter") && (b(stv, "seen.commandCenter.shown") || b(ai, "on") || b(ai, "installed"));
    let mut rs = Rows::new();
    rs.add(words(p, "Optional apps this PC runs for your devices. Each one is off until you turn it on, and can pause in Game Mode. Apps that don't come with Debian are downloaded only when you ask, from their makers, and checked before they're installed."));
    if b(sv_, "error") {
        rs.add(words_c(s(sv_, "error"), p.danger));
    }

    // ---- media & files
    rs.add(h(p, "Media & files"));
    if media.is_object() && !b(media, "installed") {
        let inst = b(media, "installing");
        let sub = if inst { app_line(media, "Jellyfin") } else { format!("{}Stream your movies, shows, music and photos to TVs, phones and Hearth devices. About 200 MB from jellyfin.org, its checksum checked", if b(media, "error") { format!("Didn't install: {}. Try again. ", s(media, "error")) } else { String::new() }) };
        rs.add(r("film", if inst { "Installing Jellyfin…" } else { "Install the media server (Jellyfin)" }).sub(sub).on_maybe((!inst).then(|| act(H::Hs, &["app-install", "jellyfin"]).ok("Downloading Jellyfin…").secs(60).msg())).el(p));
    }
    if b(media, "installed") {
        let on = b(media, "on");
        let chip = if on { if b(media, "active") { "Running" } else if b(media, "paused") { "Paused" } else { "Starting" } } else { "Off" };
        rs.add(r("film", "Media server")
            .sub(format!("{}. Jellyfin: your movies, shows, music and photos on TVs, phones and Hearth devices", app_line(media, "Jellyfin")))
            .chip(chip, if on && b(media, "active") { Ck::Ok } else { Ck::Off })
            .value(if on { if ts_only || s(media, "reach") == "tailscale" { "Tailscale only" } else { "Home and Tailscale" } } else { "" })
            .switch(p, on, Some(hs_set("mediaOn", onoff(!on), if on { "Media server off" } else { "Media server on: open it to add your shared folders as libraries" })))
            .el(p));
        let mut inner = Vec::new();
        if on {
            let reach = if ts_only { "tailscale".to_string() } else { s(media, "reach") };
            inner.push(choice(p, "", "Reachable", if ts_only { "Only through Tailscale for everything (Tailscale page)" } else { "Home network: your TVs find it. Tailscale only: just your own devices, anywhere" }, opts(&[("home", "Home network and Tailscale", ts_only), ("tailscale", "Tailscale only", false)], &reach, |v| hs_set("mediaReach", v, ""))));
            let pause = b(media, "pause");
            inner.push(r("pad", "Pause in Game Mode").sub("Stops the media server while Game Mode or a game runs (with Light mode on), and starts it again after. Off: people can keep watching while you play").switch(p, pause, Some(hs_set("mediaPause", onoff(!pause), ""))).el(p));
            for (i, u) in arr(media, "urls").iter().enumerate() {
                let u = vs(u);
                let where_ = if u.contains("localhost") { " (on this PC)" } else if u.contains(".ts.net") { " (through Tailscale)" } else { " (home network)" };
                inner.push(r("open", if i == 0 { "Open the media server" } else { "Open" }).sub(format!("{u}{where_}")).on(Msg::Open(u.clone())).el(p));
            }
        }
        inner.push(r("folder", "Photo library").sub("Your photos open in Jellyfin too: in its first steps (or Dashboard › Libraries) add your Photos folder as a \"Photos\" library. Phone photos get here with Backups & sync › Phone photos").chev().on(Msg::Go("hs-backups")).el(p));
        if !on {
            inner.push(r("trash", "Remove Jellyfin").sub("The app goes; your libraries and its settings (in /var/lib/hearth/jellyfin) stay").on(act(H::Hs, &["app-remove", "jellyfin"]).ok("Jellyfin removed. Its data stays").msg()).el(p));
        }
        rs.add(expander(p, st, "x:media", "gear", "Media server: reach, pause, open", "Who can reach it, pausing for games, opening it, your photo library", "", None, inner));
    }
    if web.is_object() {
        let on = b(web, "on");
        let sub = if on { if b(web, "active") { format!("On: your shared folders in any web browser, read only, with your user name ({}) and web password", s(stv, "user")) } else { "Starting…".into() } } else { "Open your shared folders in a web browser on any of your devices, read only, with a password".into() };
        let mut a = act(H::Hs, &["set", "webOn", onoff(!on)]);
        if !on && !b(web, "pwSet") {
            a = a.set("pwOpen", json!(true)).set("x:web", json!(true));
        }
        rs.add(r("globe", "Files on the web").sub(sub).chip(if on { if b(web, "active") { "Running" } else { "Starting" } } else { "Off" }, if on && b(web, "active") { Ck::Ok } else { Ck::Off }).switch(p, on, Some(a.msg())).el(p));
        if on {
            let mut inner = Vec::new();
            let pw = b(web, "pwSet");
            inner.push(r("lock", if pw { "Change the web password" } else { "Set a web password" }).sub(if pw { "Set. Only its hash is kept on this PC" } else { "Needed before it opens: at least 8 characters" }).on(toggle(st, "pwOpen")).el(p));
            if st.lb("pwOpen") {
                inner.push(tfield_in(p, st, "webpw", "A web password (at least 8 characters), then Enter", true));
            }
            let reach = if ts_only { "tailscale".to_string() } else if s(web, "reach").is_empty() { "tailscale".into() } else { s(web, "reach") };
            inner.push(choice(p, "", "Reachable (also share links and the drop-off)", "", opts(&[("tailscale", "Tailscale only", false), ("home", "Home network and Tailscale", ts_only)], &reach, |v| hs_set("webReach", v, ""))));
            for u in arr(web, "urls") {
                inner.push(r("open", "Open").sub(vs(u)).on(Msg::Open(vs(u))).el(p));
            }
            if arr(web, "urls").is_empty() {
                inner.push(r("globe", "Waiting for Tailscale").sub("Turn on Tailscale and sign in (Tailscale page), or pick Home network").el(p));
            }
            rs.add(expander(p, st, "x:web", "gear", "Files on the web: password, reach, open", "Its web password, who can reach it (also share links and the drop-off)", "", None, inner));
        }
    }

    // ---- AI on this PC
    rs.add(h(p, "AI on this PC"));
    if b(pic, "installing") {
        rs.add(r("image", format!("Installing ComfyUI… {}%", n(pic, "progress"))).sub(format!("{}. It keeps going if you leave this page", if s(pic, "step").is_empty() { "starting".into() } else { s(pic, "step") })).el(p));
    }
    if pic_owner_runs {
        rs.add(r("image", "Take over your own ComfyUI").sub(format!("Your ComfyUI in ~/ComfyUI runs from your own script (comfyui.service{}). Take it over: Hearth runs the same folder and models, Tailscale only, and stops it while you play. Your service is switched off, never deleted; Give it back undoes this", if b(pic, "owner.active") { ", running now" } else { "" })).value("Take over").on(act(H::Hs, &["pictures-takeover"]).ok("{note|Hearth runs your ComfyUI now}").msg()).el(p));
    }
    if pic.is_object() && !b(pic, "installed") && !b(pic, "installing") && !pic_owner_runs {
        let existing = b(pic, "existing");
        let mut sub = if b(pic, "error") { format!("Didn't install: {}. Try again. ", s(pic, "error")) } else { String::new() };
        sub.push_str(&if existing { "Hearth finishes what's missing (its Python environment, PyTorch for CUDA 12.8) and leaves your files and models where they are".to_string() } else { format!("Make pictures on the RTX graphics card from your other devices. ComfyUI {} from GitHub (its release checked), PyTorch for CUDA 12.8 from pytorch.org and what ComfyUI needs: about 10 GB, into ~/ComfyUI. No models are downloaded", s(pic, "tag")) });
        if !b(pic, "git") {
            sub.push_str(". Needs git and Python's venv (the next Hearth OS update brings them)");
        }
        rs.add(r("image", if existing { "Use your ComfyUI (~/ComfyUI)" } else { "Install AI pictures (ComfyUI)" }).badge("New").sub(sub).on(act(H::Hs, &["pictures-install"]).ok("Installing ComfyUI… it takes a while (PyTorch is a few GB)").secs(60).msg()).el(p));
    }
    if b(pic, "installed") {
        let on = b(pic, "on");
        let sub = if !on {
            format!("Off · ComfyUI {} is installed in ~/ComfyUI", s(pic, "version"))
        } else if b(pic, "paused") {
            "Stopped while you play (Light mode); it starts again after".into()
        } else if b(pic, "active") {
            if b(pic, "healthy") { format!("Running · {}", if s(pic, "device").is_empty() { format!("ComfyUI {}", s(pic, "version")) } else { s(pic, "device") }) } else { "Starting… (ComfyUI takes a moment to load)".into() }
        } else {
            "Starting…".into()
        };
        let (c, k) = run_chip(pic, b(pic, "healthy"));
        rs.add(r("image", "AI pictures").sub(sub).chip(c, k).value(if on { "Tailscale only" } else { "" }).switch(p, on, Some(hs_set("picturesOn", onoff(!on), if on { "AI pictures off" } else { "AI pictures on: only your devices through Tailscale, and this PC" }))).el(p));
        let mut inner = Vec::new();
        if on {
            inner.push(r("lock", "Who can reach it").sub(format!("Your devices through Tailscale, and this PC. Never your home network: ComfyUI listens only on this PC{}, and Hearth's firewall rule lets only Tailscale reach port 8188. ComfyUI has no password, so it stays this way", { let l = arr(pic, "listen"); if l.len() > 1 { format!(" and {}", vs(&l[1])) } else if l.len() == 1 { " (Tailscale wasn't up when it started: turn it off and on again once it is)".into() } else { " and its Tailscale address".into() } })).el(p));
            let url = s(pic, "url");
            inner.push(r("open", "Open ComfyUI").sub(if url.is_empty() { format!("On this PC: {}. Turn on Tailscale (Tailscale page) for your other devices", s(pic, "local")) } else { format!("{url} (from your devices on Tailscale) · on this PC: {}", s(pic, "local")) }).on(Msg::Open(if url.is_empty() { s(pic, "local") } else { url })).el(p));
        }
        let mf: Vec<String> = arr(pic, "modelFiles").iter().map(vs).collect();
        let msub = if mf.is_empty() { "No models yet. ".to_string() } else { format!("{}: {}{}. ", plural(mf.len() as f64, "model", "models"), mf.iter().take(3).cloned().collect::<Vec<_>>().join(", "), if mf.len() > 3 { "…" } else { "" }) };
        inner.push(r("folder", "Models folder").sub(format!("{msub}Put checkpoints (.safetensors) in ~/ComfyUI/models/checkpoints and pick one in ComfyUI's Load Checkpoint box. Hearth never downloads models")).on(Msg::Show(s(pic, "models"))).el(p));
        let video = at(pic, "video");
        if video.is_object() {
            if b(video, "installing") {
                inner.push(r("film", format!("Getting Wan 2.2 video… {}%", n(video, "progress"))).sub(format!("{}. It carries on where it stopped if it's interrupted", if s(video, "step").is_empty() { "starting".into() } else { s(video, "step") })).el(p));
            } else if !b(video, "installed") {
                inner.push(r("film", "Add video (Wan 2.2)").badge("New").sub(format!("{}Text to video and picture to video in ComfyUI: the GGUF loader and Wan 2.2 14B, sized for this graphics card (12 GB or more: the bigger files, 8 to 11 GB: smaller ones; under 8 GB it won't fit). About 45 GB, each file checked before it's used. Nothing starts by itself", if b(video, "error") { format!("Didn't finish: {}. ", s(video, "error")) } else { String::new() })).on(act(H::Hs, &["video-install"]).ok("Getting Wan 2.2… it's a big download").secs(60).msg()).el(p));
                inner.push(r("image", "Add only picture to video (Wan 2.2)").sub("The same, without text to video: about 25 GB").on(act(H::Hs, &["video-install", "i2v-only"]).ok("Getting Wan 2.2 (picture to video)…").secs(60).msg()).el(p));
            } else {
                inner.push(r("film", format!("Video: Wan 2.2 14B ({})", s(video, "quant"))).sub(format!("{}. In ComfyUI: Workflow › Browse Templates › Video › Wan 2.2 14B, with the Unet Loader (GGUF) for the two models. Other LoRAs can be added by hand in ~/ComfyUI/models/loras", if b(video, "i2vOnly") { "Picture to video" } else { "Text to video and picture to video" })).el(p));
                inner.push(r("trash", "Remove video (Wan 2.2)").sub("Only what Hearth downloaded for it goes (and the GGUF loader, if Hearth got it). Your own models, LoRAs and pictures stay").on(act(H::Hs, &["video-remove"]).ok("Wan 2.2 removed").msg()).el(p));
            }
        }
        let pause = b(pic, "pause");
        inner.push(r("pad", "Stop it while I play").sub("ComfyUI holds graphics memory: with Light mode on it stops when Game Mode or a game starts and starts again after. Off: it keeps the memory while you play").switch(p, pause, Some(hs_set("picturesPause", onoff(!pause), ""))).el(p));
        if !b(pic, "installing") {
            inner.push(r("update", "Update ComfyUI").sub(format!("To {} (the release this Hearth version checks) and its requirements. Files you changed in ComfyUI stop it instead of being written over{}", s(pic, "tag"), if s(pic, "torch").is_empty() { String::new() } else { format!(" · PyTorch {}{}", s(pic, "torch"), if s(pic, "gpu").is_empty() { " (it didn't see the graphics card)".to_string() } else { format!(" on {}", s(pic, "gpu")) }) })).on(act(H::Hs, &["pictures-update"]).ok("Updating ComfyUI…").secs(60).msg()).el(p));
        }
        if b(pic, "takenOver") {
            inner.push(r("back", "Give your own ComfyUI service back").sub("Hearth's turns off and your comfyui.service comes back on, the way it was before").on(act(H::Hs, &["pictures-giveback"]).ok("Your own ComfyUI service is back").msg()).el(p));
        }
        if !on && !pic_owner_runs {
            inner.push(r("trash", "Remove ComfyUI").sub("ComfyUI and its Python environment go. Your models, pictures (output) and workflows stay in ~/ComfyUI").on(act(H::Hs, &["pictures-remove"]).ok("ComfyUI removed. Your models and pictures stay").msg()).el(p));
            let k = st.ln("modelsRm") as i64;
            let title = match k {
                0 => "Remove ComfyUI and its models".to_string(),
                1 => "Sure? Your models will be deleted".into(),
                _ => format!("Really delete {} models? This can't be undone", mf.len()),
            };
            let sub = match k {
                0 => "Also deletes ~/ComfyUI/models (asks twice). Your pictures and workflows still stay",
                1 => "Select again to go on, or leave this page to keep them",
                _ => "Select once more to delete them now",
            };
            let m = if k < 2 { lv("modelsRm", json!(k + 1)) } else { act(H::Hs, &["pictures-remove", "models", "sure", "really-sure"]).ok("ComfyUI and its models removed").set("modelsRm", Value::Null).msg() };
            inner.push(r("trash", title).sub(sub).danger(k > 0).on(m).el(p));
        }
        rs.add(expander(p, st, "x:pictures", "gear", "AI pictures: open, models, while you play", "Who can reach it, opening ComfyUI, the models folder, updating and removing it", "", None, inner));
    }

    // ---- Voice Studio
    if b(vs_, "installing") {
        rs.add(r("vol", format!("Installing Voice Studio… {}%", n(vs_, "progress"))).sub(format!("{}. It keeps going if you leave this page", if s(vs_, "step").is_empty() { "starting".into() } else { s(vs_, "step") })).el(p));
    }
    if vs_.is_object() && !b(vs_, "installed") && !b(vs_, "installing") {
        rs.add(r("vol", "Install Voice Studio").badge("New").sub(format!("{}Your own voice studio: speak in any voice, clone a voice from a short recording, design one from words, transcribe, narrate audiobooks and dub videos, for your devices and the Command Center. About 14 GB in your Home (PyTorch, OmniVoice, Whisper, Demucs). OmniVoice's voice model is free for personal, non-commercial use (CC BY-NC); everything else is open source (Apache, MIT)", if b(vs_, "error") { format!("Didn't install: {}. Try again. ", s(vs_, "error")) } else { String::new() })).value("Install").on(act(H::Hs, &["studio-install"]).ok("Installing Voice Studio… it keeps going in the background").secs(60).msg()).el(p));
    }
    if b(vs_, "installed") {
        let on = b(vs_, "on");
        let voices = n(vs_, "voices");
        let sub = if !on {
            format!("Off · speak, clone, design, transcribe, audiobooks, dubbing{}", if voices > 0.0 { format!(" · {}", plural(voices, "voice", "voices")) } else { String::new() })
        } else if b(vs_, "paused") {
            "Stopped while you play (Light mode); its queue carries on after".into()
        } else if b(vs_, "active") {
            if b(vs_, "healthy") { format!("Running{}", if n(vs_, "queue") > 0.0 { format!(" · {} in the queue", s(vs_, "queue")) } else { String::new() }) } else { "Starting…".into() }
        } else {
            "Starting…".into()
        };
        let pinned = b(vs_, "pin") || b(vs_, "serverPin");
        let (c, k) = run_chip(vs_, b(vs_, "healthy"));
        rs.add(r("vol", "Voice Studio").sub(sub).chip(c, k).value(if on { "Tailscale only" } else { "" }).switch(p, on, Some(hs_set("studioOn", onoff(!on), if on { "Voice Studio off" } else if pinned { "Voice Studio on: only your devices through Tailscale, and this PC" } else { "Voice Studio on. Set its PIN below so you can open it" }))).el(p));
        let addr = if s(vs_, "url").is_empty() { s(vs_, "local") } else { s(vs_, "url") };
        if on {
            rs.add(r("open", "Open Voice Studio").sub(format!("{addr}{}", if pinned { " · on your phone or laptop too, through Tailscale" } else { " · set a PIN first (below)" })).on(Msg::Open(s(vs_, "local"))).el(p));
        }
        let mut inner = Vec::new();
        let open = st.lb("vsPinOpen");
        inner.push(r("lock", "PIN for Voice Studio").sub(if b(vs_, "pin") { "Its own PIN is set. A new one signs every device out; only its hash is kept" } else if b(vs_, "serverPin") { "Uses your Server mode PIN. Set one here to give Voice Studio its own" } else { "4 to 12 digits. Nobody can open the page until you set one" }).value(if b(vs_, "pin") { "●●●●●● · Change" } else { "Set" }).on(Msg::Locals(vec![("vsPinOpen".into(), if open { Value::Null } else { json!(true) })])).el(p));
        if open {
            inner.push(pinpad(p, st, "vspin", "Type a PIN of 4 to 12 digits for Voice Studio's page", 4, 12));
        }
        if b(vs_, "pin") {
            inner.push(r("back", if b(vs_, "serverPin") { "Use Server mode's PIN instead" } else { "Clear Voice Studio's PIN" }).sub(if b(vs_, "serverPin") { "Its own PIN goes; Server mode's PIN opens it" } else { "Nobody can open the page until a PIN is set again" }).on(act(H::Hs, &["studio-pin-clear"]).ok("Done").msg()).el(p));
        }
        inner.push(r("clipboard", "Address for the Command Center").sub(format!("{addr} · in Settings › Voice › Add a speech server › OpenAI-compatible speech API{}", if s(vs_, "url").is_empty() { " (turn on Tailscale for the address your other devices use)" } else { "" })).value("Copy").on(Msg::Copy(addr.clone())).el(p));
        inner.push(r("lock", "API key for apps").sub("The Command Center and other apps send it with each request. Copy it into the speech server's key field").value("Copy").on(act(H::Hs, &["studio-key"]).copy("key").ok("Key copied ✓").no_reload().msg()).el(p));
        inner.push(r("restart", "Make a new key").sub("The old key stops working: apps using it need the new one").on(act(H::Hs, &["studio-key-new"]).copy("key").ok("New key copied ✓").msg()).el(p));
        if on {
            inner.push(r("lock", "Who can reach it").sub(format!("Your devices through Tailscale, and this PC: it listens only on this PC{}, Hearth's firewall rule lets only Tailscale reach port 8890, and the page asks for the PIN", { let l = arr(vs_, "listen"); if l.len() > 1 { format!(" and {}", vs(&l[1])) } else { " and its Tailscale address".into() } })).el(p));
        }
        let pause = b(vs_, "pause");
        inner.push(r("pad", "Stop it while I play").sub("Voice Studio uses the graphics card. On: it stops when Game Mode or a game starts (with Light mode on); audiobooks and dubs waiting in its queue carry on after").switch(p, pause, Some(hs_set("studioPause", onoff(!pause), ""))).el(p));
        if !on {
            inner.push(r("trash", "Remove Voice Studio").sub("Its engines and downloaded models go (about 14 GB). Your voices, audiobooks, dubs and transcripts stay").on(act(H::Hs, &["studio-remove"]).ok("Voice Studio removed. Your voices and work are kept").msg()).el(p));
            let k = st.ln("studioRm") as i64;
            let title = match k {
                0 => "Remove Voice Studio and your voices".to_string(),
                1 => "Sure? Your voices and finished work will be deleted".into(),
                _ => format!("Really delete {} voices and your work? This can't be undone", voices),
            };
            let m = if k < 2 { lv("studioRm", json!(k + 1)) } else { act(H::Hs, &["studio-remove", "voices", "sure", "really-sure"]).ok("Voice Studio and your voices removed").set("studioRm", Value::Null).msg() };
            inner.push(r("trash", title).sub(if k == 0 { "Also deletes your cloned and designed voices, audiobooks, dubs and transcripts (asks twice)" } else { "Select again to go on, or leave this page to keep them" }).danger(k > 0).on(m).el(p));
        }
        rs.add(expander(p, st, "x:studio", "gear", "Voice Studio: PIN, apps, while you play", "The PIN for its page, the address and key for the Command Center, removing it", "", None, inner));
    }

    // ---- AI voices (Kokoro)
    if b(voi, "installing") {
        rs.add(r("wave", format!("Installing Kokoro… {}%", n(voi, "progress"))).sub(s(voi, "step")).el(p));
    }
    if b(voi, "owner.active") && !b(voi, "takenOver") {
        rs.add(r("wave", "Take over your own Kokoro").sub("Your \"kokoro\" container is running, and it answers your home network too. Take it over: it's stopped (never deleted; Give it back starts it again) and Hearth runs its own copy, Tailscale only").value("Take over").on(act(H::Hs, &["voices-takeover"]).ok("{note|Hearth runs Kokoro now}").msg()).el(p));
    }
    if voi.is_object() && !b(voi, "installed") && !b(voi, "installing") {
        let can = b(voi, "pinned") && b(voi, "podman");
        let sub = format!("{}{}", if b(voi, "error") { format!("Didn't install: {}. Try again. ", s(voi, "error")) } else { String::new() }, if !b(voi, "pinned") { "Comes with the next Hearth OS update".to_string() } else { format!("Natural voices for the Hearth AI Command Center. Kokoro-FastAPI {} for the CPU (no graphics card needed), in Podman, by the exact image Hearth pins (Podman checks it): a few GB", s(voi, "tag")) });
        rs.add(r("wave", "Install AI voices (Kokoro)").badge("New").sub(sub).on_maybe(can.then(|| act(H::Hs, &["voices-install"]).ok("Downloading Kokoro…").secs(60).msg())).el(p));
    }
    if b(voi, "installed") {
        let on = b(voi, "on");
        let sub = if !on {
            format!("Off · Kokoro-FastAPI {} is installed", if s(voi, "version").is_empty() { s(voi, "tag") } else { s(voi, "version") })
        } else if b(voi, "paused") {
            "Stopped while you play (Light mode); it starts again after".into()
        } else if b(voi, "active") {
            if b(voi, "healthy") { format!("Running · {}", if n(voi, "voices") > 0.0 { format!("{} voices", s(voi, "voices")) } else { "ready".into() }) } else { "Starting… (loading the voice model)".into() }
        } else {
            "Starting…".into()
        };
        let (c, k) = run_chip(voi, b(voi, "healthy"));
        rs.add(r("wave", "AI voices").sub(sub).chip(c, k).value(if on { "Tailscale only" } else { "" }).switch(p, on, Some(hs_set("voicesOn", onoff(!on), if on { "AI voices off" } else { "AI voices on: only your devices through Tailscale, and this PC" }))).el(p));
        let mut inner = Vec::new();
        let url = s(voi, "url");
        inner.push(r("clipboard", "Paste this into the Command Center").sub(if url.is_empty() { format!("Turn on Tailscale and sign in (Tailscale page) to get the address for your other devices. On this PC: {}", s(voi, "local")) } else { format!("{url} · in Settings › Voice › Speech servers › Kokoro-FastAPI") }).value(if url.is_empty() { "" } else { "Copy" }).on_maybe((!url.is_empty()).then(|| Msg::Copy(url.clone()))).el(p));
        if on {
            inner.push(r("lock", "Who can reach it").sub(listen_line(voi, "8880", " (Tailscale wasn't up when it started: turn it off and on again once it is)")).el(p));
        }
        let pause = b(voi, "pause");
        inner.push(r("pad", "Stop it while I play").sub("Kokoro runs on the CPU and leaves the graphics card alone, so it can keep talking while you play. On: it stops when Game Mode or a game starts (with Light mode on)").switch(p, pause, Some(hs_set("voicesPause", onoff(!pause), ""))).el(p));
        if b(voi, "takenOver") {
            inner.push(r("back", "Give your own Kokoro back").sub("Hearth's turns off and your \"kokoro\" container starts again if it was running").on(act(H::Hs, &["voices-giveback"]).ok("Your own Kokoro is back").msg()).el(p));
        }
        if !on {
            inner.push(r("trash", "Remove Kokoro").sub("Hearth's copy and its image go. Your own \"kokoro\" container, if you have one, is never touched").on(act(H::Hs, &["voices-remove"]).ok("{note|Kokoro removed}").msg()).el(p));
        }
        rs.add(expander(p, st, "x:voices", "gear", "AI voices: address, while you play", "The address for the Command Center, who can reach it, removing it", "", None, inner));
    }

    // AI chat: its own page.
    if ai_shown || b(cfg, "showHidden") {
        let chip = if b(ai, "on") { if b(ai, "active") { "Running" } else { "Starting" } } else { "" };
        rs.add(r("chat", "AI chat").sub(format!("{}. For the Hearth AI Command Center: chat models on the graphics card, reachable only through Tailscale or on this PC", app_line(ai, "Ollama"))).chip(chip, Ck::Ok).chev().on(Msg::Go("hs-ai")).el(p));
    }

    rs.add(h(p, "Also on this PC"));
    rs.add(r("play", "Remote play").sub("Sunshine: play this PC's games on your laptop, phone or TV with Moonlight, at home or through Tailscale; Use this PC from anywhere is there too").chip(if b(sv_, "remote") { "Running" } else { "" }, Ck::Ok).chev().on(Msg::Go("remote")).el(p));
    let syncs = n(sv_, "syncs");
    rs.add(r("sync", "Folder sync").sub("Syncthing: keep a folder the same on your Hearth devices, like a private Dropbox").value(if syncs > 0.0 { plural(syncs, "folder", "folders") } else { String::new() }).chev().on(Msg::Go("hs-backups")).el(p));
    rs.add(r("archive", "Game archive").sub("Hearth Storage: move games here to free space on your Hearth devices, and bring them back any time").value(if b(sv_, "storage") { "On" } else { "" }).chev().on(Msg::Go("link")).el(p));
    if at(stv, "applied.ok") == &Value::Bool(false) {
        rs.add(words_c(format!("Saved, but the system couldn't be changed to match: {}", s(stv, "applied.error")), p.danger));
    }
    let media_note = s(stv, "applied.result.apps.mediaNote");
    if b(stv, "applied.ok") && !media_note.is_empty() {
        rs.add(words_c(media_note, p.danger));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ AI chat
fn ask_model(st: &St) -> String {
    let m = st.ls("askModel");
    if !m.is_empty() {
        return m;
    }
    arr(st.get("ai"), "models").iter().find(|x| b(x, "have")).map(|x| s(x, "id")).unwrap_or_default()
}

fn ago_ms(msv: f64) -> String {
    if msv <= 0.0 {
        return "never".into();
    }
    let h_ = ((hearth_ui::time::now() as f64 * 1000.0 - msv) / 3_600_000.0).round();
    if h_ < 1.0 {
        "just now".into()
    } else if h_ < 48.0 {
        format!("{h_} h ago")
    } else {
        format!("{} days ago", (h_ / 24.0).round())
    }
}

fn svc_line(a: &Value) -> String {
    if !a.is_object() {
        return String::new();
    }
    if b(a, "installing") {
        return format!("Installing… {}%", n(a, "progress"));
    }
    if !b(a, "installed") {
        return "Not installed yet: install it in Services".into();
    }
    if !b(a, "on") {
        return "Off".into();
    }
    if b(a, "paused") {
        return "Stopped while you play; it starts again after".into();
    }
    if b(a, "active") && b(a, "healthy") { "Running".into() } else { "Starting…".into() }
}

fn model_rows<'a>(p: &Pal, models: &[&Value]) -> Vec<E<'a, Msg>> {
    models
        .iter()
        .map(|m| {
            let id = s(m, "id");
            let label = s(m, "label");
            let pulling = at(m, "pulling");
            let mut sub = format!("{} · {} · {}", s(m, "note"), id, s(m, "size"));
            if pulling.is_object() {
                sub.push_str(&format!(" · downloading {}%", s(pulling, "pct")));
            } else if b(m, "error") {
                sub.push_str(&format!(" · didn't download: {}", s(m, "error")));
            }
            let value = if b(m, "have") { if b(m, "loaded") { "Loaded".to_string() } else { "Downloaded".into() } } else if pulling.is_object() { format!("{}%", s(pulling, "pct")) } else { "Download".into() };
            let on = if pulling.is_object() {
                None
            } else if !b(m, "have") {
                Some(act_v(H::Hs, vec!["ai-pull".into(), id.clone()]).ok(format!("Downloading {label}… (Ollama checks every part)")).secs(60).msg())
            } else {
                Some(lv("askModel", json!(id.clone())))
            };
            r(if b(m, "have") { "check" } else { "plus" }, format!("{label}{}", if b(m, "recommended") { " · recommended" } else { "" })).sub(sub).value(value).on_maybe(on).el(p)
        })
        .collect()
}

pub fn ai<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let ai_ = st.get("ai");
    let svc = at(ai_, "ai");
    let cc = at(ai_, "commandCenter");
    let cc_shown = b(cc, "shown");
    let shown = cc_shown || b(svc, "on") || b(svc, "installed");
    let models = arr(ai_, "models");
    let have: Vec<&Value> = models.iter().filter(|m| b(m, "have")).collect();
    let pic = at(ai_, "pictures");
    let voi = at(ai_, "voices");
    let vs_ = at(ai_, "studio");
    let ask = ask_model(st);
    let installed = b(svc, "installed");
    let cc_days = if n(ai_, "ccDays") > 0.0 { s(ai_, "ccDays") } else { "30".into() };
    let mut rs = Rows::new();
    rs.add(words(p, "A local AI on this PC's graphics card for the Hearth AI Command Center. It answers only your own devices (through Tailscale) and this PC. Nothing leaves your home."));
    if b(ai_, "error") {
        rs.add(words_c(s(ai_, "error"), p.danger));
    }
    rs.add(r("desktops", "Share this device's AI · AI devices").sub("Let your other devices and the Command Center use this PC's AI through Tailscale, and see every device's AI in one list").chev().on(Msg::Go("hs-aidevices")).el(p));
    if ai_.is_object() && !shown {
        rs.add(r("info", "Shows once the Command Center is seen").sub(format!("These items appear when the Hearth AI Command Center was used on one of your devices in the last {cc_days} days: on this PC, or when it talks to this PC's AI service. Overview › Show hidden items shows them anyway. Hiding never turns anything off.")).el(p));
    }
    if shown {
        rs.add(h(p, "The AI service"));
        if svc.is_object() && !installed {
            let inst = b(svc, "installing");
            let sub = if inst { "From Ollama's own release on GitHub, its SHA-256 checked before anything is unpacked".to_string() } else { format!("{}About 2 GB from Ollama's GitHub release, checked against the checksum Ollama publishes. Models come separately, only the ones you pick", if b(svc, "error") { format!("Didn't install: {}. Try again. ", s(svc, "error")) } else { String::new() }) };
            rs.add(r("mic", if inst { format!("Installing Ollama… {}%", n(svc, "progress")) } else { "Install the AI service (Ollama)".into() }).sub(sub).on_maybe((!inst).then(|| act(H::Hs, &["app-install", "ollama"]).ok("Downloading Ollama…").secs(60).msg())).el(p));
        }
        if installed {
            let on = b(svc, "on");
            let loaded: Vec<String> = arr(ai_, "loaded").iter().map(vs).collect();
            let sub = if !on {
                format!("Off. Ollama {} is installed", s(svc, "version"))
            } else if b(svc, "paused") {
                "Stopped while you play (Light mode); it starts again after".into()
            } else if b(svc, "active") {
                format!("Running · Ollama {}{}", s(svc, "version"), if loaded.is_empty() { " · graphics card idle".to_string() } else { format!(" · loaded now: {}", loaded.join(", ")) })
            } else {
                "Starting…".into()
            };
            rs.add(r("mic", "AI service").sub(sub).switch(p, on, Some(hs_set("aiOn", onoff(!on), if on { "AI service off" } else { "AI service on: reachable through Tailscale and on this PC only" }))).el(p));
            if on {
                rs.add(r("lock", "Who can reach it").sub("Your devices through Tailscale, and this PC. Never your home network or the internet: Hearth's firewall rule lets only Tailscale in, and without it the service listens on this PC only").el(p));
            }
            rs.add(h(p, "Models"));
            let std_: Vec<&Value> = models.iter().filter(|m| s(m, "group") != "adult").collect();
            for e in model_rows(p, &std_) {
                rs.add(e);
            }
            rs.add(h(p, "Uncensored (18+)"));
            let adult = b(ai_, "showAdult");
            rs.add(r("lock", "Show 18+ models").sub("Uncensored chat models for adults. Off on a family PC. They're just models in Ollama: the Hearth AI Command Center's own limits still apply to anything you ask through it").switch(p, adult, Some(act(H::Hs, &["set", "showAdult", onoff(!adult)]).msg())).el(p));
            if adult {
                let ad: Vec<&Value> = models.iter().filter(|m| s(m, "group") == "adult").collect();
                for e in model_rows(p, &ad) {
                    rs.add(e);
                }
            }
            let custom = at(ai_, "custom");
            let csub = if custom.is_object() {
                format!("{}{}", s(custom, "model"), if b(custom, "error") { format!(" · didn't download: {}", s(custom, "error")) } else if b(custom, "done") { " · downloaded".into() } else { format!(" · downloading {}%", s(custom, "pct")) })
            } else {
                "From Ollama's library (qwen3:8b, user/model:tag) or a GGUF on Hugging Face (hf.co/user/repo:Q4_K_M). Downloads only when you press Enter".into()
            };
            rs.add(r("plus", "Download any model by name").sub(csub).value("Type a name").on(toggle(st, "customOpen")).el(p));
            if st.lb("customOpen") {
                rs.add(tfield_in(p, st, "custom", "A model name, then Enter to download it", false));
            }
            if !ask.is_empty() && have.iter().any(|m| s(m, "id") == ask) {
                rs.add(r("trash", format!("Remove {ask}")).sub("Frees its space; you can download it again any time. Select another downloaded model above to pick it").on(act_v(H::Hs, vec!["ai-remove".into(), ask.clone()]).ok(format!("Removed {ask}")).set("askModel", Value::Null).msg()).el(p));
            }
            let others: Vec<String> = arr(ai_, "others").iter().map(vs).collect();
            if !others.is_empty() {
                rs.add(words(p, format!("Also here: {}", others.join(", "))));
            }
        }
    }

    rs.add(h(p, "Pictures and voices"));
    if pic.is_object() {
        let mf = arr(pic, "modelFiles").len() as f64;
        rs.add(r("palette", "AI pictures (ComfyUI)").sub(format!("{}. Pictures on the graphics card, from your devices through Tailscale{}", svc_line(pic), if b(pic, "installed") { format!(" · {}", plural(mf, "model", "models")) } else { String::new() })).value(if b(pic, "on") { "On" } else { "" }).chev().on(Msg::Go("hs-services")).el(p));
        if b(pic, "installed") && b(pic, "on") {
            let url = if s(pic, "url").is_empty() { s(pic, "local") } else { s(pic, "url") };
            rs.add(r("open", "Open ComfyUI").sub(if s(pic, "url").is_empty() { format!("{} (on this PC)", s(pic, "local")) } else { s(pic, "url") }).on(Msg::Open(url)).el(p));
        }
    }
    if vs_.is_object() {
        rs.add(r("vol", "Voice Studio").sub(format!("{}. Speak, clone, design, transcribe, audiobooks and dubbing, and voices for the Command Center", svc_line(vs_))).value(if b(vs_, "on") { "On" } else { "" }).chev().on(Msg::Go("hs-services")).el(p));
        if b(vs_, "installed") && b(vs_, "on") {
            rs.add(r("open", "Open Voice Studio").sub(if s(vs_, "url").is_empty() { s(vs_, "local") } else { s(vs_, "url") }).on(Msg::Open(s(vs_, "local"))).el(p));
        }
    }
    if voi.is_object() {
        rs.add(r("vol", "AI voices (Kokoro)").sub(format!("{}. Natural voices for the Command Center, on the CPU", svc_line(voi))).value(if b(voi, "on") { "On" } else { "" }).chev().on(Msg::Go("hs-services")).el(p));
        if b(voi, "installed") {
            let url = s(voi, "url");
            rs.add(r("clipboard", "Voices address for the Command Center").sub(if url.is_empty() { format!("Turn on Tailscale for the address; on this PC: {}", s(voi, "local")) } else { format!("{url} · Settings › Voice › Speech servers › Kokoro-FastAPI") }).value(if url.is_empty() { "" } else { "Copy" }).on_maybe((!url.is_empty()).then(|| Msg::Copy(url.clone()))).el(p));
        }
    }

    if shown && !have.is_empty() && b(svc, "active") {
        rs.add(h(p, "Ask the server"));
        rs.add(tfield(p, st, "ask", format!("Ask {} something, then Enter", if ask.is_empty() { "a model".into() } else { ask.clone() }), false));
    }
    if !st.ls("answer").is_empty() {
        rs.add(words_c(st.ls("answer"), p.text));
    }

    if cc_shown {
        rs.add(h(p, "For the Command Center"));
        rs.add(r("info", format!("Seen {}{}", ago_ms(n(cc, "lastSeen")), if s(cc, "device").is_empty() { String::new() } else { format!(" on {}", s(cc, "device")) })).sub(format!("Shown because the Command Center was used on one of your devices in the last {cc_days} days")).el(p));
        let addr = s(ai_, "address");
        rs.add(r("globe", "Paste this into the Command Center").sub(if addr.is_empty() { format!("Turn on Tailscale and sign in (Tailscale page) to get the address for your other devices. On this PC: {}", s(ai_, "local")) } else { format!("{addr} · in Settings › Accounts › Your own GPU server: type \"OpenAI-compatible chat\", no key, model {}", if ask.is_empty() { "qwen3:30b-a3b".to_string() } else { ask.clone() }) }).value(if addr.is_empty() { "" } else { "Copy" }).on_maybe((!addr.is_empty()).then(|| Msg::Copy(addr.clone()))).el(p));
        let vsub = if b(vs_, "installed") {
            format!("{} in Settings › Voice › Add a speech server › OpenAI-compatible speech API, with Voice Studio's key (Services › Voice Studio){}", if s(vs_, "url").is_empty() { s(vs_, "local") } else { s(vs_, "url") }, if b(vs_, "on") { "" } else { " (turn Voice Studio on in Services)" })
        } else if b(voi, "installed") {
            format!("{} in Settings › Voice › Speech servers › Kokoro-FastAPI{}", if s(voi, "url").is_empty() { s(voi, "local") } else { s(voi, "url") }, if b(voi, "on") { "" } else { " (turn AI voices on in Services)" })
        } else {
            "Voice Studio (or AI voices, Kokoro) in Services gives the Command Center natural voices from this PC".into()
        };
        rs.add(r("vol", "Voices for the Command Center").sub(vsub).chev().on(Msg::Go("hs-services")).el(p));
    }

    if shown && installed {
        rs.add(h(p, "While you play"));
        let unload = b(svc, "unload");
        rs.add(r("pad", "Unload models when you play").sub(format!("When Game Mode or a game starts (with Light mode on), loaded models leave the graphics card within a minute; the next question loads them again{}", if n(ai_, "unloadedAt") > 0.0 { format!(" · last {}", ago_ms(n(ai_, "unloadedAt") * 1000.0)) } else { String::new() })).switch(p, unload, Some(hs_set("aiUnload", onoff(!unload), ""))).el(p));
        if b(ai_, "showAdult") && models.iter().any(|m| s(m, "id") == "huihui_ai/qwen3-abliterated:14b" && b(m, "have")) {
            let cur = if s(ai_, "smallModel").is_empty() { "qwen3:14b".into() } else { s(ai_, "smallModel") };
            rs.add(choice(p, "chat", "Small model while you play", "On the processor, so your devices still get answers while you play (Server mode's \"switch to the small model\")", opts(&[("qwen3:14b", "Qwen3 14B", false), ("huihui_ai/qwen3-abliterated:14b", "Qwen3 14B abliterated", false)], &cur, |v| hs_set("aiSmallModel", v, ""))));
        }
        let pause = b(svc, "pause");
        rs.add(r("sleep", "Stop the AI service while I play").sub("Also stops it (your other devices can't ask it then), and starts it again when you stop playing").switch(p, pause, Some(hs_set("aiPause", onoff(!pause), ""))).el(p));
        if b(svc, "active") && !arr(ai_, "loaded").is_empty() {
            rs.add(r("restart", "Unload models now").sub("Frees the graphics card right away").on(act(H::Hs, &["ai-unload"]).ok("Unloaded {unloaded}").msg()).el(p));
        }
        if !b(svc, "on") {
            rs.add(r("trash", "Remove Ollama").sub("The app goes; your downloaded models stay (in /var/lib/hearth-ollama)").on(act(H::Hs, &["app-remove", "ollama"]).ok("Ollama removed. Your models stay").msg()).el(p));
        }
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ AI devices
fn kind_word(k: &str) -> String {
    match k {
        "chat" => "Chat".into(),
        "voice" => "Voices".into(),
        "image" => "Pictures".into(),
        "stt" => "Speech to text".into(),
        other => other.to_string(),
    }
}
fn reason_word(r_: &str) -> &'static str {
    match r_ {
        "off" => "Not shared",
        "playing" => "Not while you play",
        "battery" => "Waiting for a charger",
        "nothing" => "No AI found here",
        "offline" => "Offline",
        "not-sharing" => "Not sharing its AI",
        _ => "",
    }
}
fn svc_ai_line(sv_: &Value, here: bool) -> String {
    let mut bits = Vec::new();
    let models: Vec<String> = arr(sv_, "models").iter().map(vs).collect();
    if !models.is_empty() {
        bits.push(format!("{}{}", models.iter().take(3).cloned().collect::<Vec<_>>().join(", "), if models.len() > 3 { format!(" +{}", models.len() - 3) } else { String::new() }));
    }
    let voices = arr(sv_, "voices").len();
    if voices > 0 {
        bits.push(format!("{voices} voices"));
    }
    let head_ = if here {
        if bits.is_empty() { format!("Found on port {}", s(sv_, "port")) } else { bits.join(", ") }
    } else {
        format!("{} ({}){}", kind_word(&s(sv_, "kind")), s(sv_, "engine"), if bits.is_empty() { String::new() } else { format!(": {}", bits.join(", ")) })
    };
    let tail = if !b(sv_, "healthy") { " · not answering" } else if at(sv_, "reachable") == &Value::Bool(false) { " · only on that device" } else if b(sv_, "busy") { " · busy" } else { "" };
    format!("{head_}{tail}")
}
fn dev_line(d: &Value) -> String {
    let mut bits = vec![if b(d, "online") { "Online".to_string() } else { "Offline".into() }];
    if b(d, "online") && b(d, "gaming") {
        bits.push("playing".into());
    }
    if b(d, "battery") {
        bits.push(format!("{}%{}", s(d, "battery.percent"), if b(d, "pluggedIn") { " · plugged in" } else { " · on battery" }));
    } else if b(d, "online") {
        bits.push("plugged in".into());
    }
    if b(d, "gpu") {
        bits.push(match s(d, "gpu.vendor").as_str() {
            "nvidia" => "Nvidia card".into(),
            "amd" => "AMD graphics".into(),
            _ => "Intel graphics".into(),
        });
    }
    let sv_: Vec<String> = arr(d, "services").iter().map(|x| svc_ai_line(x, false)).collect();
    format!("{}{}", bits.join(" · "), if !sv_.is_empty() { format!("\n{}", sv_.join("\n")) } else if s(d, "via") == "hearth link" { "\nPaired over Hearth Link; turn on Share this device's AI and Tailscale on it".into() } else { String::new() })
}

pub fn aidevices<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let sh = st.get("share");
    let cfg = at(sh, "settings");
    let has_cfg = cfg.is_object();
    let shown = list_shown(st);
    let list = st.get("list");
    let set = |k: &str, v: bool, ok: &str| act(H::AiShare, &["set", k, onoff(v)]).ok(format!("{{note|{}}}", if ok.is_empty() { "Saved" } else { ok })).msg();
    let mut rs = Rows::new();
    rs.add(words(p, "Each of your Hearth devices runs its own AI jobs: big chat and pictures on the PC with the strongest card, voices or speech to text on the others. Share a device's AI and your other devices, and the Hearth AI Command Center, can use it through Tailscale. Nothing is shared until you turn it on."));
    if b(sh, "error") {
        rs.add(words_c(s(sh, "error"), p.danger));
    }
    if has_cfg {
        rs.add(h(p, "Share this device's AI"));
        let on = b(cfg, "on");
        let reason = s(sh, "reason");
        let sub = if !on {
            "Off: nothing is shared".to_string()
        } else if !b(sh, "running") {
            "Starting…".into()
        } else if reason == "playing" {
            "Paused while you play: your devices use another one".into()
        } else if reason == "battery" {
            "Paused on battery: shares again when plugged in".into()
        } else if reason == "nothing" {
            "On, but no AI service was found on this device".into()
        } else if !b(sh, "tailscale") {
            "On this device only: turn on Tailscale (Home server › Tailscale) for your other devices".into()
        } else if !b(sh, "firewall") && !b(cfg, "lan") {
            "Waiting for Hearth's firewall rule: on this device only until it loads".into()
        } else {
            format!("Shared with your devices through Tailscale as {}", s(sh, "device"))
        };
        rs.add(r("mic", "Share this device's AI").sub(sub).switch(p, on, Some(set("on", !on, if on { "Not sharing this device's AI" } else { "Sharing this device's AI with your devices" }))).el(p));
        if on {
            rs.add(r("lock", "Who can see it").sub(if b(cfg, "lan") { "Your devices through Tailscale, this device, and everything on your home network" } else { "Your devices through Tailscale, and this device. Never your home network or the internet: Hearth's firewall rule lets only Tailscale reach it. It's a read-only list of what's here, with no keys or files" }).el(p));
        }
        rs.add(h(p, "What to share"));
        let found = arr(sh, "found");
        if found.is_empty() {
            rs.add(r("info", "No AI services found on this device").sub("Hearth looks for Ollama chat (port 11434), Kokoro voices (8880), ComfyUI pictures (8188) and whisper speech to text (8000 or 9000). On a home server, Home server › AI installs Ollama").el(p));
        }
        for f in found {
            let kind = s(f, "kind");
            let ic = match kind.as_str() {
                "voice" => "vol",
                "image" => "palette",
                "stt" => "mic",
                _ => "grid",
            };
            let shared = nf(cfg, &format!("share.{kind}"));
            let label = s(f, "label");
            rs.add(r(ic, label.clone()).sub(format!("{}{}", svc_ai_line(f, true), if b(f, "reachable") { "" } else { " · it listens on this device only, so other devices can't use it yet" })).switch(p, shared, Some(set(&kind, !shared, &if shared { format!("Not shared: {label}") } else { format!("Shared: {label}") }))).el(p));
        }
        rs.add(h(p, "When to share"));
        if b(sh, "battery") {
            let po = b(cfg, "pluggedOnly");
            rs.add(r("power", "Share only when plugged in").sub(format!("On battery, your devices use another one · now {}%{}", s(sh, "battery.percent"), if b(sh, "pluggedIn") { ", plugged in" } else { ", on battery" })).switch(p, po, Some(set("pluggedOnly", !po, ""))).el(p));
        }
        let nwp = b(cfg, "notWhilePlaying");
        rs.add(r("pad", "Not while I'm playing").sub(format!("While Game Mode or a game runs, this device says it's playing and your devices use another one{}", if b(sh, "gaming") { " · playing now" } else { "" })).switch(p, nwp, Some(set("notWhilePlaying", !nwp, ""))).el(p));
        if on {
            let lan = b(cfg, "lan");
            rs.add(r("wired", "Also on my home network").sub("Off: only Tailscale and this device. On: anything on your home network can read the list too (the services keep their own rules)").switch(p, lan, Some(set("lan", !lan, ""))).el(p));
        }
        if on || b(cfg, "hub") {
            let code_on = b(cfg, "codeOn");
            let m = if !code_on && !b(sh, "codeSet") {
                lv("codeField", json!(true))
            } else if !code_on {
                set("codeOn", true, "")
            } else {
                act(H::AiShare, &["code-clear"]).ok("No share code any more").msg()
            };
            rs.add(r("lock", "Ask for a share code").sub(if b(sh, "codeSet") { "The Command Center sends it (X-Hearth-Share-Code). Only its hash is kept here. Use the same code on all your devices" } else { "Off. Select to set a code (8 to 64 characters); only its hash is kept" }).switch(p, code_on, Some(m)).el(p));
        }
        if b(sh, "server") || b(cfg, "hub") {
            rs.add(h(p, "Your AI devices"));
            let hub = b(cfg, "hub");
            rs.add(r("globe", "Let the Command Center find my devices here").sub("This home server lists every device's AI at one address, so the Command Center needs only that").switch(p, hub, Some(set("hub", !hub, if hub { "Not listing your devices any more" } else { "Listing your devices' AI for the Command Center" }))).el(p));
            if hub {
                let ccad = s(sh, "commandCenter");
                rs.add(r("globe", "Command Center address").sub(if ccad.is_empty() { format!("Turn on Tailscale and sign in (Home server › Tailscale) to get the address. On this device: {}", s(sh, "local")) } else { format!("{ccad} · paste it into the Command Center once: it finds every device from there") }).value(if ccad.is_empty() { "" } else { "Copy" }).on_maybe((!ccad.is_empty()).then(|| Msg::Copy(ccad.clone()))).el(p));
                let router = b(cfg, "router");
                rs.add(r("globe", "One AI address").sub(if router && !ccad.is_empty() { format!("Use {ccad}/v1 in any app that speaks OpenAI's API: chat, voices and speech to text go to the first free device in the order below") } else { "Any app that speaks OpenAI's API uses this server's address, and each request goes to the first free device that has the model".into() }).switch(p, router, Some(set("router", !router, if router { "One AI address is off" } else { "One AI address is on" }))).el(p));
            }
        }
    }
    if shown {
        let devs = arr(list, "devices");
        if !list.is_object() {
            rs.add(r("update", "Looking for your devices…").el(p));
        }
        if !devs.is_empty() {
            rs.add(words(p, "In the order the Command Center tries them. Select a device to move it up or down."));
        }
        let pk = st.ls("pick");
        for d in devs {
            let dev = s(d, "device");
            let value = if !b(d, "online") { "Offline".to_string() } else if b(d, "gaming") { "Playing".into() } else if b(d, "busy") { "Busy".into() } else if b(d, "available") { "Ready".into() } else { reason_word(&s(d, "reason")).to_string() };
            let title = format!("{}. {}{}{}", s(d, "rank"), if s(d, "name").is_empty() { dev.clone() } else { s(d, "name") }, if s(d, "via") == "this device" { " (this device)" } else { "" }, if pk == dev { "  ◂ selected" } else { "" });
            rs.add(r(if pk == dev { "check" } else if b(d, "battery") { "batt" } else { "display" }, title).sub(dev_line(d)).value(value).on(pick(st, "pick", &dev)).el(p));
        }
        if !pk.is_empty() {
            let lr = rd("list", H::AiShare, &["devices"]);
            rs.add(r("back", format!("Move {pk} up")).on(act_v(H::AiShare, vec!["move".into(), pk.clone(), "up".into()]).no_reload().also(lr.clone()).msg()).el(p));
            rs.add(r("forward", format!("Move {pk} down")).on(act_v(H::AiShare, vec!["move".into(), pk.clone(), "down".into()]).no_reload().also(lr).msg()).el(p));
        }
        if !arr(cfg, "priority").is_empty() {
            rs.add(r("restart", "Back to the usual order").sub("A device with an Nvidia card first, then the others by graphics memory").on(act(H::AiShare, &["order-reset"]).ok("Usual order").msg()).el(p));
        }
    } else if has_cfg {
        rs.add(r("info", "Every device in one list").sub("On your Home server, this page lists every Hearth device's AI and gives the Command Center its address").el(p));
    }
    rs.add(message(p, st));
    if st.lb("codeField") {
        rs.add(tfield(p, st, "code", "Type a share code (8 to 64 characters), then Enter", true));
    }
    rs.el()
}

// ------------------------------------------------------------------ Server apps
fn line(a: &Value) -> String {
    if b(a, "installing") {
        return format!("Downloading… {}% · {}", n(a, "progress"), if s(a, "step").is_empty() { "starting".into() } else { s(a, "step") });
    }
    if b(a, "error") && !b(a, "installed") {
        return format!("Didn't install: {}. Try again", s(a, "error"));
    }
    if !b(a, "installed") {
        return format!("{} · {}", s(a, "desc"), s(a, "size"));
    }
    if !b(a, "on") {
        return "Off · installed".into();
    }
    if b(a, "failed") {
        return "Stopped: it couldn't start (Logs below say why). Turn it off and on to try again".into();
    }
    if !b(a, "running") {
        return "Starting…".into();
    }
    let addrs = arr(a, "addresses");
    format!("Running · {}{}", if s(a, "reach") == "home" { "your home network and Tailscale" } else { "Tailscale and this PC only" }, if addrs.is_empty() { String::new() } else { format!(" · {}", s(&addrs[0], "url")) })
}

fn wago(t_: f64) -> String {
    let d = hearth_ui::time::local(t_ as i64);
    let now = hearth_ui::time::local(hearth_ui::time::now());
    let y = hearth_ui::time::local(hearth_ui::time::now() - 86_400);
    let hm = format!("{:02}:{:02}", d.hour, d.min);
    if d.year == now.year && d.mon == now.mon && d.day == now.day {
        format!("today {hm}")
    } else if d.year == y.year && d.mon == y.mon && d.day == y.day {
        format!("last night {hm}")
    } else {
        format!("{} {hm}", hearth_ui::time::day(t_ as i64))
    }
}
fn mb(x: f64) -> String {
    if x >= 1e9 { format!("{:.1} GB", x / 1e9) } else { format!("{} MB", (x / 1e6).round().max(1.0)) }
}
fn cap(c: &str) -> String {
    match c {
        "on" => "On".into(),
        "off" => "Off".into(),
        other => {
            let mut ch = other.chars();
            match ch.next() {
                Some(f) => format!("{}{}", f, ch.as_str().to_lowercase()),
                None => String::new(),
            }
        }
    }
}

fn app_card<'a>(p: &Pal, st: &St, a: &Value) -> Vec<E<'a, Msg>> {
    let id = s(a, "id");
    let name = s(a, "name");
    let ic = s(a, "icon");
    let podman = b(st.get("list"), "podman");
    let mut out = Vec::new();
    let aok = |args: Vec<String>, ok: String| act_v(H::Apps, args).ok(if ok.is_empty() { "{note|Done ✓}".to_string() } else { ok }).secs(120).msg();
    if !b(a, "installed") {
        let inst = b(a, "installing");
        out.push(r(&ic, format!("{}{name}", if inst { "Installing " } else { "Install " })).badge("New").sub(line(a)).value(if inst { format!("{}%", s(a, "progress")) } else { "Install".into() }).on_maybe((!inst && podman).then(|| aok(vec!["install".into(), id.clone()], format!("Downloading {name}…")))).el(p));
        return out;
    }
    let on = b(a, "on");
    let (chip, k) = if !on { ("Off", Ck::Off) } else if b(a, "failed") { ("Stopped", Ck::Warn) } else if b(a, "running") { ("Running", Ck::Ok) } else { ("Starting", Ck::Off) };
    out.push(r(&ic, name.clone()).sub(line(a)).chip(chip, k).switch(p, on, Some(aok(vec!["set".into(), id.clone(), "on".into(), onoff(!on).into()], format!("{name} {}", onoff(!on))))).el(p));
    let mut inner = Vec::new();
    for ad in arr(a, "addresses") {
        inner.push(r("link", s(ad, "where")).sub(s(ad, "url")).value("Copy").on(Msg::Copy(s(ad, "url"))).el(p));
    }
    if has(a, "eula") {
        let e = at(a, "eula") == &Value::Bool(true);
        inner.push(r("doc", "I accept Minecraft's EULA").sub(format!("Mojang's rules for running a server ({}). Needed before it can start", s(a, "eulaUrl"))).switch(p, e, Some(aok(vec!["set".into(), id.clone(), "eula".into(), onoff(!e).into()], String::new()))).el(p));
    }
    let id2 = id.clone();
    inner.push(choice(p, "", "Who can reach it", "", opts(&[("home", "Home network and Tailscale", false), ("tailscale", "Tailscale only", false)], &s(a, "reach"), move |v| {
        act_v(H::Apps, vec!["set".into(), id2.clone(), "reach".into(), v.into()]).ok(if v == "home" { "Your home network and Tailscale can reach it" } else { "Only Tailscale and this PC can reach it" }).msg()
    })));
    for o in arr(a, "opts") {
        let key = s(o, "key");
        let label = s(o, "label");
        let choices_: Vec<String> = arr(o, "choices").iter().map(vs).collect();
        if !choices_.is_empty() {
            let cur = s(o, "value");
            let items = choices_.iter().map(|c| (cap(c), *c == cur, Some(act_v(H::Apps, vec!["opt".into(), id.clone(), key.clone(), c.clone()]).ok(format!("Saved ✓{}", if on { format!(" · restarting {name}") } else { String::new() })).msg()))).collect();
            inner.push(choice(p, "", label.clone(), "", items));
        }
        if b(o, "text") {
            inner.push(tfield_in(p, st, &format!("opt:{id}:{key}"), format!("{label}: {} (type a new one, then Enter)", s(o, "value")), false));
        }
        if b(o, "folder") {
            let then = act_v(H::Apps, vec!["opt".into(), id.clone(), key.clone(), "{path}".into()]).ok("Folder saved ✓");
            inner.push(r("folder", label.clone()).sub(s(o, "value")).value("Change").on(act(H::Hs, &["pick-folder"]).secs(600).no_reload().then_if("path", then).msg()).el(p));
        }
    }
    if a.get("password").map(|x| x.is_object()).unwrap_or(false) {
        let pw = at(a, "password");
        let ph = format!("{}{}", if b(pw, "set") { "Password set · type a new one, then Enter".to_string() } else { format!("A password for joining (at least {} characters), then Enter", s(pw, "min")) }, if b(pw, "required") { "" } else { " · empty clears it" });
        inner.push(tfield_in(p, st, &format!("pw:{id}"), ph, true));
    }
    inner.push(r("folder", "Its data").sub(s(a, "data")).value("Open").on(Msg::Open(s(a, "data"))).el(p));
    if on {
        let logs = st.get(&format!("log:{id}"));
        let lines: Vec<String> = arr(logs, "lines").iter().map(vs).collect();
        let shown = if lines.is_empty() { if logs.is_object() { if b(logs, "error") { s(logs, "error") } else { "Nothing yet".into() } } else { "The last lines it printed".into() } } else { lines[lines.len().saturating_sub(6)..].join("\n") };
        inner.push(r("file", "Logs").sub(shown).value("Show").on(act_v(H::Apps, vec!["logs".into(), id.clone()]).store(&format!("log:{id}")).no_reload().msg()).el(p));
    }
    inner.push(r("update", format!("Update {name}")).sub(format!("Downloads its newest image{}; your data stays", if on { " and restarts it" } else { "" })).on_maybe((!b(a, "installing")).then(|| aok(vec!["update".into(), id.clone()], format!("Updating {name}…")))).el(p));
    if !on {
        inner.push(r("trash", format!("Remove {name}")).sub(format!("Its images go; your data stays in {}", s(a, "data"))).on(aok(vec!["remove".into(), id.clone()], String::new())).el(p));
        let asking = st.ls("removing") == id;
        let m = if !asking { lv("removing", json!(id.clone())) } else { act_v(H::Apps, vec!["remove".into(), id.clone(), "data".into(), "sure".into(), "really-sure".into()]).ok("{note|Done ✓}").set("removing", Value::Null).msg() };
        inner.push(r("trash", if asking { "Really remove its data too?".to_string() } else { format!("Remove {name} and its data") }).danger(true).sub(if asking { format!("Press again: {} is deleted for good", s(a, "data")) } else { format!("Worlds, saves and everything in {} (asks twice)", s(a, "data")) }).on(m).el(p));
    }
    let optnames: Vec<String> = arr(a, "opts").iter().map(|o| { let l = s(o, "label").to_lowercase(); l.split(" (").next().unwrap_or("").to_string() }).collect();
    let sub = format!("Who can reach it, {}logs, update, remove", if optnames.is_empty() { String::new() } else { format!("{}, ", optnames.join(", ")) });
    out.push(expander(p, st, &format!("x:{id}"), "gear", format!("{name}: address, options"), sub, "", None, inner));
    out
}

pub fn apps<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let lst = st.get("list");
    let worlds = st.get("worlds");
    let all = arr(lst, "apps");
    let games: Vec<&Value> = all.iter().filter(|a| s(a, "kind") == "game").collect();
    let own: Vec<&Value> = all.iter().filter(|a| s(a, "kind") == "app").collect();
    let mut rs = Rows::new();
    rs.add(words(p, "Game servers for you and your friends, and your own versions of apps you'd otherwise pay for. Each one is off until you install it and turn it on, runs in Podman as your own service, and never holds up starting this PC or Game Mode. Your worlds and data live in Home › Server apps."));
    if lst.is_object() && (b(lst, "error") || at(lst, "podman") == &Value::Bool(false)) {
        rs.add(words_c(if b(lst, "error") { s(lst, "error") } else { "Podman isn't on this PC: server apps come with the Hearth OS update that brings it.".into() }, p.danger));
    }
    if !games.is_empty() {
        rs.add(h(p, "Game servers"));
        for a in &games {
            for e in app_card(p, st, a) {
                rs.add(e);
            }
        }
    }
    if worlds.is_object() && !b(worlds, "error") && games.iter().any(|a| b(a, "installed")) {
        rs.add(h(p, "World backups"));
        let won = b(worlds, "settings.on");
        rs.add(r("cloud", "Back up the worlds every night").sub("Each game server is stopped cleanly, its world copied to Home › Server apps › World backups, then started again. Also before every update. A server with players on waits an hour").switch(p, won, Some(act(H::Apps, &["worlds-set", "on", onoff(!won)]).ok(if won { "World backups off" } else { "World backups on" }).msg())).el(p));
        if won {
            let cur = s(worlds, "settings.at");
            rs.add(choice(p, "", "At", "", opts(&[("02:30", "02:30", false), ("03:30", "03:30", false), ("04:30", "04:30", false), ("05:30", "05:30", false), ("06:30", "06:30", false)], &cur, |v| act(H::Apps, &["worlds-set", "at", v]).ok(format!("World backups at {v}")).msg())));
            let keep = s(worlds, "settings.keep");
            rs.add(choice(p, "", "Keep them for", "", opts(&[("3", "3 days", false), ("7", "7 days", false), ("14", "14 days", false), ("30", "30 days", false)], &keep, |v| act(H::Apps, &["worlds-set", "keep", v]).ok(format!("Kept for {v} days (the newest always stays)")).msg())));
        }
    }
    for w in arr(worlds, "servers") {
        let id = s(w, "id");
        let name = s(w, "name");
        let backups = arr(w, "backups");
        let sub = if b(w, "error") { format!("The last one didn't finish: {}", s(w, "error")) } else if !backups.is_empty() { format!("{} kept · newest {}", backups.len(), wago(n(&backups[0], "at"))) } else { "None yet".into() };
        let mut inner = vec![r("cloud", "Back up now").sub(format!("Stops {name} cleanly for a moment, then starts it again")).on(act_v(H::Apps, vec!["world-backup".into(), id.clone()]).ok(format!("Backing up {name}…")).secs(600).msg()).el(p)];
        for (i, bk) in backups.iter().enumerate() {
            let file = s(bk, "file");
            let key = format!("{id}/{file}");
            let rolling = st.ls("rolling") == key;
            let when_ = wago(n(bk, "at"));
            let title = if rolling { "Press again to roll back".to_string() } else if i == 0 && s(bk, "why") == "nightly" && when_.starts_with("last night") { "Roll back to last night".into() } else { format!("Roll back to {when_}") };
            let m = if !rolling { lv("rolling", json!(key)) } else { act_v(H::Apps, vec!["world-restore".into(), id.clone(), file.clone()]).ok(format!("{name} rolled back to {when_} ✓")).set("rolling", Value::Null).secs(600).msg() };
            inner.push(r("undo", title).danger(rolling).sub(format!("{}{} · {}", if rolling { "Today's world is kept as a backup first, then this one is put back · " } else { "" }, s(bk, "why"), mb(n(bk, "size")))).value("Roll back").on(m).el(p));
        }
        rs.add(expander(p, st, &format!("w:{id}"), "pad", format!("{name}: world backups"), sub, "", None, inner));
    }
    if !own.is_empty() {
        rs.add(h(p, "Your own apps"));
        for a in &own {
            for e in app_card(p, st, a) {
                rs.add(e);
            }
        }
    }
    if lst.is_object() {
        rs.add(h(p, "Also"));
        rs.add(r("bell", "Phone notifications").sub("Hear on your phone (the ntfy app) when an alert comes up or a server app stops").chev().on(Msg::Go("hs-phone")).el(p));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Phone notifications
pub fn phone<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let nt = st.get("ntfy");
    let cfg = at(nt, "settings");
    let set = |k: &str, v: &str, ok: &str| act(H::Apps, &["ntfy-set", k, v]).store("ntfy").no_reload().ok(ok).msg();
    let mut rs = Rows::new();
    rs.add(words(p, "Get a notification on your phone when something on this server needs you, wherever you are. Install the free ntfy app (Android or iPhone), make a topic here, and subscribe to it in the app."));
    if b(nt, "error") {
        rs.add(words_c(s(nt, "error"), p.danger));
    }
    if cfg.is_object() {
        let topic = !s(cfg, "topic").is_empty();
        let url = s(nt, "url");
        let mut g = Vec::new();
        g.push(r("bell", if topic { "Subscribe to this in the ntfy app" } else { "Make a topic" })
            .sub(if url.is_empty() { "A private, random name your phone subscribes to. Nothing is sent until you turn notifications on".to_string() } else { format!("{url} · anyone with this name can read it, so it's long and random") })
            .value(if topic { "Copy" } else { "Make one" })
            .on(if !url.is_empty() { Msg::Copy(url.clone()) } else { set("topic", "new", "Topic made: subscribe to it in the ntfy app") })
            .el(p));
        if topic {
            g.push(r("restart", "Make a new topic").sub("If someone else got the old one. Subscribe to the new one on your phone afterwards").on(set("topic", "new", "New topic made: subscribe to it on your phone")).el(p));
        }
        let server = s(cfg, "server");
        g.push(r("globe", "Server").sub(format!("{server}{} · your own ntfy is in Server apps", if server == "https://ntfy.sh" { " (free, run by ntfy's maker)" } else { "" })).value("Change").on(toggle(st, "serverOpen")).el(p));
        if st.lb("serverOpen") {
            g.push(tfield_in(p, st, "server", "https://ntfy.sh, or your own like http://this-pc:2586, then Enter", false));
        }
        rs.add(group(p, "Your topic", g));
        let on = b(cfg, "on");
        let alerts = b(cfg, "alerts");
        let apps_ = b(cfg, "apps");
        let mut g = Vec::new();
        g.push(r("bell", "Send notifications to my phone").sub(if !topic { "Make a topic first" } else { "Checked every 5 minutes; the same problem is sent again after a day if it's still there" }).switch(p, on, Some(set("on", onoff(!on), if on { "Phone notifications off" } else { "Phone notifications on" }))).el(p));
        g.push(r("warn", "Home server alerts").sub("What Alerts & power finds: a drive failing or very hot, a drive filling up, a backup failing or late, a mirror missing a drive (turn alerts on there too)").switch(p, alerts, Some(set("alerts", onoff(!alerts), ""))).el(p));
        g.push(r("server", "Server apps that stop").sub("A game server or app that stopped and couldn't start again").switch(p, apps_, Some(set("apps", onoff(!apps_), ""))).el(p));
        if topic {
            let last = at(nt, "last");
            g.push(r("check", "Send a test").sub(if last.is_object() { if b(last, "ok") { "The last one went out".to_string() } else { format!("The last one didn't go out: {}", s(last, "error")) } } else { "One notification now, to check your phone gets it".into() }).on(act(H::Apps, &["ntfy-test"]).ok("Sent ✓ Check your phone").msg()).el(p));
        }
        rs.add(group(p, "Notifications", g));
    }
    rs.add(message(p, st));
    rs.el()
}

// ------------------------------------------------------------------ Gaming
pub fn gaming<'a>(p: &Pal, st: &'a St) -> E<'a, Msg> {
    let g = st.get("gaming");
    let ok = g.is_object() && !b(g, "error");
    let mut rs = Rows::new();
    rs.add(words(p, "Your games, mods and installers in one place for every Hearth device, and what this PC holds back while you play."));
    if b(g, "error") {
        rs.add(words_c(s(g, "error"), p.danger));
    }
    rs.add(h(p, "Install from home"));
    if ok && !b(g, "games") {
        rs.add(r("folder", "Share a Games folder").sub("Home › Games, read only for your devices: put game installers (GOG, itch.io), mods and emulator files there and copy them to any Hearth device at home-network speed").on(act(H::Hs, &["folder-share", "games"]).ok("Games folder shared ✓ (Shared folders has its settings)").msg()).el(p));
    }
    if b(g, "games") {
        let gm = at(g, "games");
        rs.add(r("folder", "Games folder").sub(format!("\\\\{}\\{} · {} · {}", if s(g, "tsName").is_empty() { "this PC".into() } else { s(g, "tsName") }, s(gm, "name"), if b(gm, "write") { "read & write" } else { "read only" }, s(gm, "path"))).chev().on(Msg::Go("hs-shares")).el(p));
    }
    rs.add(r("store", "Steam downloads at home").sub("Steam copies games between your PCs and Steam Decks on the same network by itself (Steam › Settings › Downloads › Local Network Game Transfers, on each device). Hearth never signs in to Steam or changes its settings").el(p));
    rs.add(r("desktops", "Game archive and saves").sub("Hearth Storage keeps games for your Hearth devices (download once, copy at home speed) and lets saves follow you").value(if b(g, "storage") { "On" } else { "" }).chev().on(Msg::Go("link")).el(p));
    rs.add(h(p, "Play from anywhere"));
    rs.add(r("wired", "Remote play and Use this PC from anywhere").sub(format!("Sunshine on this PC, Moonlight on your laptop, work PC, phone or TV: your games, or your whole desktop. Away from home, add this PC in Moonlight by its Tailscale name{}: nothing is opened to the internet", if s(g, "tsName").is_empty() { String::new() } else { format!(" ({})", s(g, "tsName")) })).value(if b(g, "remote") { "On" } else { "" }).chev().on(Msg::Go("remote")).el(p));
    rs.add(r("user", "Friends over Tailscale").sub("To play your home-network games with a friend somewhere else, share this PC with them from Tailscale's admin page (Share node). They only reach this PC, nothing else at home").el(p));
    rs.add(h(p, "Light mode while you play"));
    if ok {
        let lm = b(g, "lightMode");
        let sub = if b(g, "playing") { if b(g, "resumed") { "You're playing; paused jobs were let through for now (Quick Access)" } else { "You're playing now: the jobs below wait" } } else { "When Game Mode or a game runs, big jobs wait. Also in Game Mode's Quick Access" };
        rs.add(r("sleep", "Light mode while you play").sub(sub).switch(p, lm, Some(act(H::Hs, &["set", "lightMode", onoff(!lm)]).msg())).el(p));
        let waits = |x: bool| if x { "Waits" } else { "Keeps running" };
        let items: Vec<(String, String)> = vec![
            ("Waits".into(), format!("Snapshots and emptying recycle bins{}", if b(g, "snapshots") { "" } else { " (snapshots are off)" })),
            ("Waits".into(), format!("Backups{}: they run at the next hour after you stop", if n(g, "backups") > 0.0 { format!(" ({} set up)", s(g, "backups")) } else { " (none set up)".into() })),
            (waits(b(g, "ai.on") && b(g, "ai.unload")).into(), format!("AI models: {}", if b(g, "ai.on") { if b(g, "ai.pause") { "the AI service stops while you play" } else if b(g, "ai.unload") { "unloaded from the graphics card within a minute" } else { "stay loaded (AI › While you play)" } } else { "the AI service is off" })),
            (waits(b(g, "media.on") && b(g, "media.pause")).into(), format!("The media server: {}", if b(g, "media.on") { if b(g, "media.pause") { "stops while you play" } else { "keeps streaming (Services › Pause in Game Mode)" } } else { "off" })),
            (waits(b(g, "pictures.on") && b(g, "pictures.pause")).into(), format!("AI pictures (ComfyUI): {}", if b(g, "pictures.on") { if b(g, "pictures.pause") { "stops while you play, so the graphics card is yours, and starts again after" } else { "keeps running and holds graphics memory (Services › Stop while you play)" } } else { "off" })),
            (waits(b(g, "voices.on") && b(g, "voices.pause")).into(), format!("AI voices (Kokoro): {}", if b(g, "voices.on") { if b(g, "voices.pause") { "stops while you play" } else { "keeps running on the CPU" } } else { "off" })),
            ("Keeps running".into(), format!("Shared folders, Tailscale, folder sync{}, Files on the web and links", if n(g, "syncs") > 0.0 { format!(" ({})", s(g, "syncs")) } else { String::new() })),
        ];
        for (t_, s_) in items {
            rs.add(r(if t_ == "Waits" { "clock" } else { "check" }, t_).sub(s_).el(p));
        }
    }
    rs.add(words(p, "\"Let paused jobs run now\" in Quick Access lets them through for up to 4 hours without changing Light mode."));
    rs.add(message(p, st));
    rs.el()
}

#[cfg(test)]
mod tests {
    #[test]
    fn model_names() {
        for ok in ["qwen3:8b", "huihui_ai/qwen3-abliterated:14b", "hf.co/unsloth/Qwen3-30B-A3B-GGUF:Q4_K_M", "llama3"] {
            assert!(super::model_name_ok(ok), "{ok}");
        }
        for bad in ["", "Qwen3", "a/b/c", "hf.co/only", "x..y", "qwen3:"] {
            assert!(!super::model_name_ok(bad), "{bad}");
        }
    }
}
