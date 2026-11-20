//! Hearth Player's window (Tauri: WebKitGTK on Linux, WebView2 on Windows). The page (ui/) plays files from the
//! library's local server; long jobs (edits, getting a file ready to play, podcast downloads) report back as
//! "player://job" events and finish with "player://job-done". MPRIS presses arrive as "player://remote".
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Hearth's look (Settings › Appearance), shared by Hearth's apps.
#[path = "../../hearth-apptheme/apptheme.rs"]
mod apptheme;
use hearth_player::{awake, media, net, server::Server, store, tags, tools};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const USAGE: &str = "Hearth Player: videos, music, podcasts and radio.

Usage: hearth-player [options] [files, folders, playlists or web addresses...]

  --fullscreen     start full screen
  --music          start in the music view
  --podcasts       start in Podcasts & Radio
  --disc [PATH]    play a disc: a drive (/dev/sr0), a .cue, an .iso or a VIDEO_TS folder
                   (no PATH: the first drive with a disc in it)
  --show WHAT      open with a screen showing (disc, controller), for pictures of it
  --version        print the version
  -h, --help       this help
";

struct App {
    server: Arc<Server>,
    tools: Mutex<tools::Tools>,
    launch: Value,
    jobs: Mutex<HashMap<u32, tools::Cancel>>,
    next_job: AtomicU32,
    #[cfg(target_os = "linux")]
    mpris: tokio::sync::OnceCell<Option<hearth_player::mpris::Mpris>>,
    #[cfg(target_os = "linux")]
    discs: Mutex<HashMap<String, Arc<hearth_player::discplay::Opened>>>,
    /// The DVD menu running (one at a time): the disc's id and its session.
    #[cfg(target_os = "linux")]
    menu: Mutex<Option<(String, Arc<hearth_player::dvdnav::MenuSession>)>>,
}

fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }

#[tauri::command(async)]
fn init(app: tauri::State<Arc<App>>) -> Value {
    let t = app.tools.lock().unwrap().clone();
    json!({
        "launch": app.launch,
        "platform": if cfg!(windows) { "windows" } else { "linux" },
        "version": env!("CARGO_PKG_VERSION"),
        "tools": t,
        "settings": store::load("settings"),
        "places": hearth_player::places(),
        "podcastDir": hearth_player::podcast_dir(),
        "home": dirs::home_dir(),
    })
}

/// Files, folders and playlists as one play queue.
#[tauri::command(async)]
fn open_paths(paths: Vec<String>) -> Vec<media::Item> { media::expand(&paths) }

#[tauri::command(async)]
fn folder_tracks(path: String) -> Vec<media::Item> { media::folder_audio(Path::new(&path)) }

#[tauri::command(async)]
fn list_dir(path: String, hidden: Option<bool>, filter: Option<String>) -> Result<Vec<hearth_player::Entry>, String> {
    hearth_player::list_dir(Path::new(&path), hidden.unwrap_or(false), filter.as_deref().unwrap_or("media"))
}

/// The address the page plays a file (or a web stream) from.
#[tauri::command]
fn media_url(app: tauri::State<Arc<App>>, path: String) -> Result<String, String> {
    if path.starts_with("http://") || path.starts_with("https://") { return Ok(app.server.proxy_url(&path)); }
    let p = PathBuf::from(&path);
    if !p.is_file() { return Err(format!("{} isn't there any more.", p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(path))); }
    Ok(app.server.file_url(&p))
}

#[tauri::command]
fn proxy_url(app: tauri::State<Arc<App>>, url: String) -> String { app.server.proxy_url(&url) }

/// Tracks, chapters and tags (ffprobe), the subtitle files beside it, and the song's tags (Lofty) when there's no ffprobe.
#[tauri::command(async)]
fn probe(app: tauri::State<Arc<App>>, path: String) -> Value {
    let p = PathBuf::from(&path);
    let t = app.tools.lock().unwrap().clone();
    let info = tools::probe(&t, &p);
    let song = if media::kind(&p) == Some("audio") { tags::read(&p).ok() } else { None };
    json!({
        "info": info.as_ref().ok(),
        "error": info.as_ref().err(),
        "subs": media::sidecar_subs(&p),
        "tags": song,
    })
}

/// A subtitle file beside the video, or a text subtitle stream inside it (ffmpeg), as WebVTT text (the page draws it).
#[tauri::command(async)]
fn subtitle_vtt(app: tauri::State<Arc<App>>, path: String, stream: Option<u32>) -> Result<String, String> {
    let p = PathBuf::from(&path);
    match stream {
        Some(i) => { let t = app.tools.lock().unwrap().clone(); tools::extract_subtitle(&t, &p, i) }
        None => media::to_vtt(&p),
    }
}

/// A subtitle file picked by hand (any name), as WebVTT text.
#[tauri::command(async)]
fn subtitle_file_vtt(path: String) -> Result<String, String> { media::to_vtt(Path::new(&path)) }

/// The song's cover: the picture inside it, else folder.jpg/cover.jpg beside it.
#[tauri::command(async)]
fn art_url(app: tauri::State<Arc<App>>, path: String) -> Option<String> {
    let p = PathBuf::from(&path);
    if let Some((bytes, mime)) = tags::cover(&p) { return Some(app.server.bytes_url(bytes, &mime, "cover")); }
    let dir = if p.is_dir() { p.clone() } else { p.parent()?.to_path_buf() };
    media::cover_file(&dir).map(|c| app.server.file_url(&c))
}

#[tauri::command(async)]
fn tags_read(path: String) -> Result<tags::Tags, String> { tags::read(Path::new(&path)) }

#[tauri::command(async)]
fn tags_write(path: String, tags: tags::Tags, cover: Option<tags::CoverChange>) -> Result<tags::Tags, String> {
    let p = PathBuf::from(&path);
    tags::write(&p, &tags, &cover.unwrap_or_default())?;
    tags::read(&p)
}

#[tauri::command(async)]
fn store_get(name: String) -> Value { store::load(&name) }

#[tauri::command(async)]
fn store_set(name: String, value: Value) -> Result<(), String> { store::save(&name, &value) }

/// "video", "audio" or "": keep the screen (or just the computer) awake while playing.
#[tauri::command(async)]
fn keep_awake(mode: String) -> String { awake::set(&mode) }

/// What's playing, for MPRIS (media keys, Mantel's lock screen).
#[tauri::command]
async fn now_playing(app: tauri::State<'_, Arc<App>>, state: Value) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let st: hearth_player::mpris::State = serde_json::from_value(state).map_err(err)?;
        if let Some(Some(m)) = app.mpris.get() { m.update(st).await.map_err(err)?; }
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, state); }
    Ok(())
}

#[tauri::command]
fn fullscreen(window: tauri::WebviewWindow, on: bool) -> Result<bool, String> {
    window.set_fullscreen(on).map_err(err)?;
    window.is_fullscreen().map_err(err)
}

#[tauri::command]
fn set_title(window: tauri::WebviewWindow, title: String) { let _ = window.set_title(&title); }

/// The text of a feed or a directory's answer (the page can't read other sites itself).
#[tauri::command(async)]
fn net_get(url: String) -> Result<String, String> { net::get_text(&url, 16 * 1024 * 1024) }

/// Saves a frame: the PNG the page drew (base64), or one ffmpeg takes from the file.
#[tauri::command(async)]
fn save_frame(app: tauri::State<Arc<App>>, path: String, at: f64, png: Option<String>) -> Result<String, String> {
    use base64::Engine;
    let video = PathBuf::from(&path);
    let out = hearth_player::screenshot_path(&video, at);
    match png {
        Some(b64) => {
            let data = b64.split_once(',').map(|x| x.1).unwrap_or(&b64);
            let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(err)?;
            std::fs::write(&out, bytes).map_err(|e| format!("Couldn't save the picture: {e}"))?;
        }
        None => {
            let t = app.tools.lock().unwrap().clone();
            let ff = t.ffmpeg.ok_or("ffmpeg isn't installed")?;
            let o = std::process::Command::new(ff).args(tools::frame_args(&video, at, &out)).output().map_err(err)?;
            if !o.status.success() || !out.exists() { return Err("Couldn't take the picture.".into()); }
        }
    }
    Ok(out.to_string_lossy().into_owned())
}

fn new_job(app: &App) -> (u32, tools::Cancel) {
    let id = app.next_job.fetch_add(1, Ordering::SeqCst) + 1;
    let c = tools::Cancel::default();
    app.jobs.lock().unwrap().insert(id, c.clone());
    (id, c)
}

fn spawn_job(handle: tauri::AppHandle, app: Arc<App>, id: u32, kind: &'static str, work: impl FnOnce(&dyn Fn(f64, &str)) -> Result<Value, String> + Send + 'static) {
    std::thread::spawn(move || {
        let h2 = handle.clone();
        let progress = move |pct: f64, text: &str| { let _ = h2.emit("player://job", json!({"id": id, "kind": kind, "pct": pct, "text": text})); };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&progress))).unwrap_or_else(|_| Err("Something went wrong inside Hearth Player.".into()));
        app.jobs.lock().unwrap().remove(&id);
        let _ = handle.emit("player://job-done", match r {
            Ok(v) => json!({"id": id, "kind": kind, "ok": true, "result": v}),
            Err(e) => json!({"id": id, "kind": kind, "ok": false, "error": e}),
        });
    });
}

#[tauri::command]
fn job_cancel(app: tauri::State<Arc<App>>, id: u32) {
    if let Some(c) = app.jobs.lock().unwrap().get(&id) { c.0.store(true, Ordering::SeqCst); }
}

/// What an edit would do (without doing it): for the summary before saving.
#[tauri::command(async)]
fn edit_plan(app: tauri::State<Arc<App>>, edit: tools::Edit) -> Result<Value, String> {
    let t = app.tools.lock().unwrap().clone();
    let p = tools::probe(&t, Path::new(&edit.input))?;
    let plan = tools::plan(&edit, &p, &t.encoders, &std::env::temp_dir())?;
    Ok(json!({"output": plan.output, "reencode": plan.reencode, "how": plan.how, "duration": plan.duration}))
}

/// Saves an edit as a new file.
#[tauri::command]
fn edit_start(handle: tauri::AppHandle, app: tauri::State<Arc<App>>, edit: tools::Edit) -> Result<u32, String> {
    let app = app.inner().clone();
    let (id, cancel) = new_job(&app);
    let t = app.tools.lock().unwrap().clone();
    spawn_job(handle, app, id, "edit", move |progress| {
        progress(0.0, "Getting ready");
        let p = tools::probe(&t, Path::new(&edit.input))?;
        let tmp = hearth_player::cache_dir().join(format!("edit-{}-{id}", std::process::id()));
        let plan = tools::plan(&edit, &p, &t.encoders, &tmp)?;
        let how = plan.how.clone();
        let out = tools::run(&t, &plan, &|f| progress(f, &how), &cancel);
        let _ = std::fs::remove_dir_all(&tmp);
        let out = out?;
        if plan.audio { let _ = tags::copy(Path::new(&edit.input), &out); }
        Ok(json!({"path": out, "how": how}))
    });
    Ok(id)
}

/// Makes a copy the web view can play (a file it couldn't, or another sound track). Done straight away when the
/// copy was made before.
#[tauri::command]
fn prepare_start(handle: tauri::AppHandle, app: tauri::State<Arc<App>>, path: String, audio: Option<u32>) -> Result<u32, String> {
    let app = app.inner().clone();
    let (id, cancel) = new_job(&app);
    let t = app.tools.lock().unwrap().clone();
    spawn_job(handle, app, id, "prepare", move |progress| {
        let input = PathBuf::from(&path);
        let out = hearth_player::cache_dir().join("playable").join(format!("{}.mp4", hearth_player::file_key(&input, &format!("a{}", audio.unwrap_or(0)))));
        if out.is_file() { return Ok(json!({"path": out, "cached": true})); }
        std::fs::create_dir_all(out.parent().unwrap()).map_err(err)?;
        // Keep the cache from growing without end: the oldest copies go past 8 GB.
        trim_cache(&out.parent().unwrap().to_path_buf(), 8 << 30);
        let p = tools::probe(&t, &input)?;
        let plan = tools::prepare_plan(&p, &input, &out, audio, &t.encoders)?;
        let how = format!("Getting it ready to play ({})", if plan.how.is_empty() { "copying" } else { &plan.how });
        let r = tools::run(&t, &plan, &|f| progress(f, &how), &cancel)?;
        Ok(json!({"path": r, "cached": false}))
    });
    Ok(id)
}

fn trim_cache(dir: &PathBuf, max: u64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = rd.flatten().filter_map(|e| {
        let m = e.metadata().ok()?;
        Some((m.modified().ok()?, m.len(), e.path()))
    }).collect();
    files.sort();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    for (_, len, p) in files {
        if total <= max { break; }
        if std::fs::remove_file(&p).is_ok() { total -= len; }
    }
}

/// Downloads a podcast episode into Music/Podcasts/<show>/.
#[tauri::command]
fn download_start(handle: tauri::AppHandle, app: tauri::State<Arc<App>>, url: String, show: String, title: String) -> Result<u32, String> {
    let app = app.inner().clone();
    let (id, cancel) = new_job(&app);
    spawn_job(handle, app, id, "download", move |progress| {
        let ext = url.split(['?', '#']).next().unwrap_or("").rsplit('.').next().filter(|e| e.len() <= 4 && e.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or("mp3").to_ascii_lowercase();
        let dest = hearth_player::podcast_dir().join(net::safe_file_name(&show)).join(format!("{}.{ext}", net::safe_file_name(&title)));
        if dest.is_file() { return Ok(json!({"path": dest})); }
        net::download(&url, &dest, &|got, total| {
            let mb = got as f64 / 1048576.0;
            match total {
                Some(t) if t > 0 => progress(got as f64 / t as f64, &format!("{mb:.1} of {:.1} MB", t as f64 / 1048576.0)),
                _ => progress(0.5, &format!("{mb:.1} MB")),
            }
        }, &cancel.0)?;
        Ok(json!({"path": dest}))
    });
    Ok(id)
}

#[tauri::command(async)]
fn delete_download(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    // Only episodes this app downloaded.
    if !p.starts_with(hearth_player::podcast_dir()) { return Err("Only downloaded episodes can be removed here.".into()); }
    std::fs::remove_file(&p).map_err(err)
}

/// Saves the play queue as a playlist.
#[tauri::command(async)]
fn save_playlist(path: String, items: Vec<String>) -> Result<String, String> {
    let mut p = PathBuf::from(&path);
    if p.extension().is_none() { p.set_extension("m3u8"); }
    if p.exists() { return Err("A file with that name is already there.".into()); }
    std::fs::write(&p, media::write_m3u(&items)).map_err(err)?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command(async)]
fn file_exists(path: String) -> bool { Path::new(&path).exists() }

// ------------------------------------------------------------------ CDs and DVDs (src/disc.rs, dvd.rs, discplay.rs)

/// The CD/DVD drives and what's in them.
#[tauri::command(async)]
fn disc_drives() -> Value {
    #[cfg(target_os = "linux")]
    { json!(hearth_player::disc::drives()) }
    #[cfg(not(target_os = "linux"))]
    { json!([]) }
}

/// Opens a disc (a drive, a .cue, an .iso or a disc's folder): its titles, chapters, tracks and names.
#[tauri::command(async)]
fn disc_open(app: tauri::State<Arc<App>>, path: Option<String>) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        use hearth_player::discplay;
        let path = match path.filter(|p| !p.is_empty()) { Some(p) => p, None => discplay::default_source().ok_or(discplay::EMPTY_NOTE)? };
        let o = Arc::new(discplay::open(&path)?);
        let info = json!(o.info);
        let mut d = app.discs.lock().unwrap();
        for (_, old) in d.drain() { old.forget(&app.server); }
        d.insert(o.info.id.clone(), o);
        Ok(info)
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, path); Err("Discs play on Hearth OS (Linux) for now.".into()) }
}

/// Starts title (DVD) or track (CD, VCD) `n` at `at` seconds, with a sound track and subtitles (DVD stream ids).
#[tauri::command(async)]
fn disc_play(app: tauri::State<Arc<App>>, id: String, n: u32, at: Option<f64>, audio: Option<u32>, sub: Option<u32>, reencode: Option<bool>) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        let o = app.discs.lock().unwrap().get(&id).cloned().ok_or("The disc isn't open any more.")?;
        let t = app.tools.lock().unwrap().clone();
        let p = hearth_player::discplay::play(&o, &app.server, t.ffmpeg.as_deref(), &t.encoders, n, at.unwrap_or(0.0), audio, sub, reencode.unwrap_or(false))?;
        Ok(json!(p))
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, id, n, at, audio, sub, reencode); Err("Discs play on Hearth OS (Linux) for now.".into()) }
}

/// Starts (or, with `again`, restarts the picture of) the disc's own DVD menu: libdvdnav runs it, ffmpeg turns its
/// picture into JPEG frames. The menu's buttons come back as "player://dvdmenu" events, and so does the title a button
/// plays. An error means the disc's menu can't run (the titles screen stands in).
#[tauri::command(async)]
fn disc_menu_start(handle: tauri::AppHandle, app: tauri::State<Arc<App>>, id: String, again: Option<bool>) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        use hearth_player::dvdnav::{menu_args, MenuSession};
        let o = app.discs.lock().unwrap().get(&id).cloned().ok_or("The disc isn't open any more.")?;
        if !o.info.menu { return Err("This disc's own menu can't run here.".into()); }
        let ff = app.tools.lock().unwrap().ffmpeg.clone().ok_or("The disc's menu needs ffmpeg, which isn't installed.")?;
        let mut cur = app.menu.lock().unwrap();
        let reuse = again.unwrap_or(false) && cur.as_ref().map(|(i, _)| *i == id).unwrap_or(false);
        let session = if reuse {
            let s = cur.as_ref().unwrap().1.clone();
            s.stop();
            s
        } else {
            if let Some((_, s)) = cur.take() { s.stop(); }
            let h = handle.clone();
            let s = MenuSession::open(Path::new(&o.info.source), Box::new(move |v| { let _ = h.emit("player://dvdmenu", v); }))
                .map_err(|e| format!("The disc's menu didn't start: {e}"))?;
            *cur = Some((id.clone(), s.clone()));
            s
        };
        drop(cur);
        let vurl = app.server.stream_url(session.clone(), "menu.vob");
        let url = app.server.live_url(hearth_player::server::Live { program: ff, args: menu_args(&vurl, &session.palette(), session.spu()), mime: "multipart/x-mixed-replace;boundary=ffmpeg".into() }, "menu.mjpeg");
        Ok(json!({"url": url, "menu": session.state()}))
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (handle, app, id, again); Err("Not here".into()) }
}

/// A move or press in the disc's menu: up, down, left, right, activate, hover/click (x, y on the menu's picture),
/// root, title.
#[tauri::command(async)]
fn disc_menu_cmd(app: tauri::State<Arc<App>>, cmd: String, x: Option<i32>, y: Option<i32>) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        let s = app.menu.lock().unwrap().as_ref().map(|(_, s)| s.clone()).ok_or("The disc's menu isn't running.")?;
        Ok(json!(s.command(&cmd, x.unwrap_or(0), y.unwrap_or(0))))
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, cmd, x, y); Err("Not here".into()) }
}

/// Leaves the disc's menu (it stops reading the disc).
#[tauri::command(async)]
fn disc_menu_stop(app: tauri::State<Arc<App>>) {
    #[cfg(target_os = "linux")]
    if let Some((_, s)) = app.menu.lock().unwrap().take() { s.stop(); }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

/// An audio CD's names from the internet (MusicBrainz, then CDDB).
#[tauri::command(async)]
fn disc_names(app: tauri::State<Arc<App>>, id: String) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        let o = app.discs.lock().unwrap().get(&id).cloned().ok_or("The disc isn't open any more.")?;
        let toc = o.toc().ok_or("Only music CDs have names to look up.")?;
        Ok(json!(hearth_player::disc::lookup_names(toc)?))
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, id); Err("Not here".into()) }
}

/// Opens the tray (and forgets the disc).
#[tauri::command(async)]
fn disc_eject(app: tauri::State<Arc<App>>, device: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let mut d = app.discs.lock().unwrap();
        d.retain(|_, o| { let same = o.info.device.as_deref() == Some(device.as_str()); if same { o.forget(&app.server); } !same });
        drop(d);
        hearth_player::disc::eject(&device)
    }
    #[cfg(not(target_os = "linux"))]
    { let _ = (app, device); Err("Not here".into()) }
}

/// Whether DVD decoding (libdvdcss) is installed, and whether Hearth Player can add it on this system.
#[tauri::command(async)]
fn dvd_decoding() -> Value {
    #[cfg(target_os = "linux")]
    {
        let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let fedora = os.lines().any(|l| l == "ID=fedora" || l == "ID=\"fedora\"" || (l.starts_with("ID_LIKE=") && l.contains("fedora")));
        let helper = Path::new("/usr/libexec/nimbus/hearth-dvd-decoding").exists();
        json!({"installed": hearth_player::dvd::have_dvdcss(), "canAdd": (fedora && helper) || std::env::var_os("HEARTH_PLAYER_DVD_DECODING_DEMO").is_some()})
    }
    #[cfg(not(target_os = "linux"))]
    { json!({"installed": false, "canAdd": false}) }
}

/// Adds (or removes) DVD decoding: libdvdcss from RPM Fusion's free-tainted repository, through hearth-admin (which
/// asks for an administrator's password). Only after you said yes in the player. Progress as a job.
#[tauri::command]
fn dvd_decoding_change(handle: tauri::AppHandle, app: tauri::State<Arc<App>>, add: bool) -> Result<u32, String> {
    let app = app.inner().clone();
    let (id, _cancel) = new_job(&app);
    spawn_job(handle, app, id, "dvd-decoding", move |progress| {
        use std::io::BufRead;
        progress(0.02, if add { "Adding DVD decoding" } else { "Removing DVD decoding" });
        let demo = std::env::var_os("HEARTH_PLAYER_DVD_DECODING_DEMO").is_some();
        let mut child = (if demo { std::process::Command::new("sh").args(["-c", "echo 'progress: 50 Downloading'; echo '{\"ok\": true, \"live\": true}'"]).stdout(std::process::Stdio::piped()).spawn() }
            else { std::process::Command::new("pkexec").args(["/usr/libexec/nimbus/hearth-admin", if add { "dvd-decoding-add" } else { "dvd-decoding-remove" }]).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn() })
            .map_err(|e| format!("Couldn't start it: {e}"))?;
        let out = child.stdout.take().unwrap();
        let mut last = Value::Null;
        let mut error = String::new();
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("progress: ") {
                let (pct, what) = rest.split_once(' ').unwrap_or((rest, ""));
                progress(pct.parse::<f64>().unwrap_or(0.0) / 100.0, what);
            } else if let Some(e) = line.strip_prefix("error: ") { error = e.to_string(); }
            else if line.starts_with('{') { last = serde_json::from_str(&line).unwrap_or(Value::Null); }
        }
        let st = child.wait().map_err(err)?;
        if st.code() == Some(126) || st.code() == Some(127) { return Err("It wasn't allowed (the password was cancelled).".into()); }
        if !st.success() { return Err(if error.is_empty() { "It didn't work.".into() } else { error }); }
        Ok(last)
    });
    Ok(id)
}

#[tauri::command]
fn quit(app: tauri::AppHandle) { awake::set(""); app.exit(0); }

/// WebKit's newer drawing path shows a black window on older Nvidia drivers and on basic graphics (the same rule
/// as Hearth Terminal and the Hearth app).
#[cfg(target_os = "linux")]
fn pick_drawing() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_some() { return; }
    let fallback = dirs::config_dir().map(|d| d.join("nimbus").join("drawing-fallback").exists()).unwrap_or(false)
        || std::fs::read_to_string("/proc/cmdline").map(|c| c.split_whitespace().any(|w| w == "nomodeset")).unwrap_or(false);
    let nvidia = std::fs::read_dir("/sys/class/drm").into_iter().flatten().flatten()
        .any(|e| std::fs::read_to_string(e.path().join("device/vendor")).map(|v| v.trim() == "0x10de").unwrap_or(false));
    // Nvidia: WebKit's newer drawing path (DMA-BUF) crashes these windows on 615.x too, so the older path is used
    // on every Nvidia driver, as Hearth Text, Clock and the other apps already do.
    let _new_driver = std::fs::read_to_string("/sys/module/nvidia/version").unwrap_or_default()
        .trim().split('.').next().and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 560).unwrap_or(false);
    if fallback || nvidia { std::env::set_var(VAR, "1"); }
}

#[cfg(target_os = "linux")]
fn is_disc(p: &Path) -> bool { hearth_player::disc::looks_like_disc(p) }
#[cfg(not(target_os = "linux"))]
fn is_disc(_: &Path) -> bool { false }

fn main() {
    let mut files = Vec::new();
    let (mut full, mut view) = (false, "player");
    let mut disc: Option<String> = None;
    let mut show = String::new();
    let mut args = std::env::args().skip(1).peekable();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--disc" => { disc = Some(match args.peek() { Some(n) if !n.starts_with("--") => { let n = args.next().unwrap(); if n.starts_with('/') { n } else { std::env::current_dir().map(|d| d.join(&n).to_string_lossy().into_owned()).unwrap_or(n) } } _ => String::new() }); }
            "--show" => { show = args.next().unwrap_or_default(); }
            "-h" | "--help" => { print!("{USAGE}"); return; }
            "--version" => { println!("hearth-player {}", env!("CARGO_PKG_VERSION")); return; }
            "--fullscreen" => full = true,
            "--music" => view = "music",
            "--podcasts" => view = "podcasts",
            s if s.starts_with("--") => { eprintln!("hearth-player: unknown option {s}\n\n{USAGE}"); std::process::exit(2); }
            _ => {
                let p = Path::new(&a);
                let full_path = if a.contains("://") || p.is_absolute() { a.clone() } else { std::env::current_dir().map(|d| d.join(p).to_string_lossy().into_owned()).unwrap_or(a.clone()) };
                // A drive, a disc image or a disc's folder (Plasma's "Play disc" passes the drive or the mounted folder).
                if disc.is_none() && is_disc(Path::new(&full_path)) { disc = Some(full_path); continue; }
                files.push(full_path);
            }
        }
    }
    #[cfg(target_os = "linux")]
    pick_drawing();
    let server = Server::start().expect("couldn't start the local file server");
    let app = Arc::new(App {
        server,
        tools: Mutex::new(tools::detect()),
        launch: json!({"files": files, "view": view, "fullscreen": full, "disc": disc, "show": show}),
        jobs: Mutex::new(HashMap::new()),
        next_job: AtomicU32::new(0),
        #[cfg(target_os = "linux")]
        mpris: tokio::sync::OnceCell::new(),
        #[cfg(target_os = "linux")]
        discs: Mutex::new(HashMap::new()),
        #[cfg(target_os = "linux")]
        menu: Mutex::new(None),
    });

    tauri::Builder::default().plugin(apptheme::plugin())
        .manage(app.clone())
        .invoke_handler(tauri::generate_handler![
            init, open_paths, folder_tracks, list_dir, media_url, proxy_url, probe, subtitle_vtt, subtitle_file_vtt, art_url,
            tags_read, tags_write, store_get, store_set, keep_awake, now_playing, fullscreen, set_title, net_get, save_frame,
            job_cancel, edit_plan, edit_start, prepare_start, download_start, delete_download, save_playlist, file_exists, quit,
            disc_drives, disc_open, disc_play, disc_names, disc_eject, dvd_decoding, dvd_decoding_change,
            disc_menu_start, disc_menu_cmd, disc_menu_stop
        ])
        .setup(move |a| {
            let w = WebviewWindowBuilder::new(a, "main", WebviewUrl::App("index.html".into()))
                .title("Hearth Player")
                .inner_size(1180.0, 720.0)
                .min_inner_size(480.0, 320.0)
                .fullscreen(full)
                .background_color(apptheme::bg())
                .build()?;
            let _ = w.set_focus();
            // A disc going in or out: udev tells (a thread asleep on its socket; nothing is checked over and over).
            #[cfg(target_os = "linux")]
            {
                let h = a.handle().clone();
                hearth_player::disc::watch(move |dev| { let _ = h.emit("player://disc", json!({"device": dev})); });
            }
            #[cfg(target_os = "linux")]
            {
                let handle = a.handle().clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let h = handle.clone();
                    let cb: hearth_player::mpris::Cb = Arc::new(move |cmd: &str, v: Option<f64>, s: Option<String>| {
                        match cmd {
                            "raise" => { if let Some(w) = h.get_webview_window("main") { let _ = w.unminimize(); let _ = w.set_focus(); } }
                            "quit" => { awake::set(""); h.exit(0); }
                            _ => { let _ = h.emit("player://remote", json!({"cmd": cmd, "value": v, "uri": s})); }
                        }
                    });
                    let m = match hearth_player::mpris::Mpris::start(cb).await {
                        Ok(m) => Some(m),
                        Err(e) => { eprintln!("hearth-player: no MPRIS (media keys): {e}"); None }
                    };
                    let _ = app.mpris.set(m);
                });
            }
            Ok(())
        })
        .on_window_event(|_, e| { if let tauri::WindowEvent::Destroyed = e { awake::set(""); } })
        .run(tauri::generate_context!())
        .expect("couldn't start Hearth Player");
}
