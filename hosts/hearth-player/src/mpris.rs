//! MPRIS (org.mpris.MediaPlayer2 on the session bus): how Mantel's media keys and lock screen (Quickshell's Mpris
//! service), KDE Connect, headset buttons and every other Linux media control see and drive Hearth Player.
//! The page sends what's playing (`update`); presses come back through `Cb` as short command names.

use std::collections::HashMap;
use std::sync::Arc;
use zbus::object_server::SignalContext;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

pub type Cb = Arc<dyn Fn(&str, Option<f64>, Option<String>) + Send + Sync>;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct State {
    /// "Playing", "Paused" or "Stopped".
    pub status: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: String,
    pub url: String,
    /// Seconds.
    pub length: f64,
    pub position: f64,
    pub can_next: bool,
    pub can_prev: bool,
    pub rate: f64,
    pub volume: f64,
    /// Changes with every new file (MPRIS wants a track id).
    pub track: u64,
    /// The page jumped (seeking): tell listeners.
    pub seeked: bool,
}

const PATH: &str = "/org/mpris/MediaPlayer2";

fn ov<'a>(v: impl Into<Value<'a>>) -> OwnedValue { OwnedValue::try_from(v.into()).expect("plain values have no file descriptors") }

struct Root { cb: Cb }

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) { (self.cb)("raise", None, None) }
    fn quit(&self) { (self.cb)("quit", None, None) }
    #[zbus(property)]
    fn can_quit(&self) -> bool { true }
    #[zbus(property)]
    fn can_raise(&self) -> bool { true }
    #[zbus(property)]
    fn has_track_list(&self) -> bool { false }
    #[zbus(property)]
    fn identity(&self) -> String { "Hearth Player".into() }
    #[zbus(property)]
    fn desktop_entry(&self) -> String { "hearth-player".into() }
    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> { vec!["file".into(), "http".into(), "https".into()] }
    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        ["video/mp4", "video/x-matroska", "video/webm", "video/quicktime", "audio/mpeg", "audio/flac", "audio/ogg", "audio/mp4", "audio/x-wav", "audio/opus"].iter().map(|s| s.to_string()).collect()
    }
}

struct Player { cb: Cb, st: State }

impl Player {
    fn track_path(&self) -> ObjectPath<'static> {
        ObjectPath::try_from(format!("/io/github/dragongamer9/hearth/player/track/{}", self.st.track)).unwrap_or_else(|_| ObjectPath::from_static_str_unchecked("/org/mpris/MediaPlayer2/TrackList/NoTrack"))
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn next(&self) { (self.cb)("next", None, None) }
    fn previous(&self) { (self.cb)("previous", None, None) }
    fn pause(&self) { (self.cb)("pause", None, None) }
    fn play_pause(&self) { (self.cb)("playpause", None, None) }
    fn stop(&self) { (self.cb)("stop", None, None) }
    fn play(&self) { (self.cb)("play", None, None) }
    fn seek(&self, offset: i64) { (self.cb)("seekby", Some(offset as f64 / 1e6), None) }
    fn set_position(&self, _track_id: ObjectPath<'_>, position: i64) { (self.cb)("seekto", Some(position as f64 / 1e6), None) }
    fn open_uri(&self, uri: String) { (self.cb)("open", None, Some(uri)) }

    #[zbus(property)]
    fn playback_status(&self) -> String { if self.st.status.is_empty() { "Stopped".into() } else { self.st.status.clone() } }
    #[zbus(property)]
    fn rate(&self) -> f64 { if self.st.rate > 0.0 { self.st.rate } else { 1.0 } }
    #[zbus(property)]
    fn set_rate(&mut self, r: f64) { (self.cb)("rate", Some(r), None) }
    #[zbus(property)]
    fn minimum_rate(&self) -> f64 { 0.25 }
    #[zbus(property)]
    fn maximum_rate(&self) -> f64 { 4.0 }
    #[zbus(property)]
    fn volume(&self) -> f64 { self.st.volume }
    #[zbus(property)]
    fn set_volume(&mut self, v: f64) { (self.cb)("volume", Some(v.clamp(0.0, 1.0)), None) }
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 { (self.st.position * 1e6) as i64 }
    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let mut m = HashMap::new();
        m.insert("mpris:trackid".to_string(), ov(self.track_path()));
        if self.st.length > 0.0 { m.insert("mpris:length".into(), ov((self.st.length * 1e6) as i64)); }
        if !self.st.title.is_empty() { m.insert("xesam:title".into(), ov(self.st.title.clone())); }
        if !self.st.artist.is_empty() { m.insert("xesam:artist".into(), ov(vec![self.st.artist.clone()])); }
        if !self.st.album.is_empty() { m.insert("xesam:album".into(), ov(self.st.album.clone())); }
        if !self.st.art_url.is_empty() { m.insert("mpris:artUrl".into(), ov(self.st.art_url.clone())); }
        if !self.st.url.is_empty() { m.insert("xesam:url".into(), ov(self.st.url.clone())); }
        m
    }
    #[zbus(property)]
    fn can_go_next(&self) -> bool { self.st.can_next }
    #[zbus(property)]
    fn can_go_previous(&self) -> bool { self.st.can_prev }
    #[zbus(property)]
    fn can_play(&self) -> bool { true }
    #[zbus(property)]
    fn can_pause(&self) -> bool { true }
    #[zbus(property)]
    fn can_seek(&self) -> bool { self.st.length > 0.0 }
    #[zbus(property)]
    fn can_control(&self) -> bool { true }

    #[zbus(signal)]
    async fn seeked(ctxt: &SignalContext<'_>, position: i64) -> zbus::Result<()>;
}

pub struct Mpris { conn: zbus::Connection }

impl Mpris {
    pub async fn start(cb: Cb) -> zbus::Result<Mpris> {
        let name = format!("org.mpris.MediaPlayer2.HearthPlayer.instance{}", std::process::id());
        let conn = zbus::connection::Builder::session()?
            .name(name)?
            .serve_at(PATH, Root { cb: cb.clone() })?
            .serve_at(PATH, Player { cb, st: State::default() })?
            .build()
            .await?;
        Ok(Mpris { conn })
    }

    pub async fn update(&self, st: State) -> zbus::Result<()> {
        let iface = self.conn.object_server().interface::<_, Player>(PATH).await?;
        let ctxt = iface.signal_context().clone();
        let mut p = iface.get_mut().await;
        let old = std::mem::replace(&mut p.st, st);
        let new = p.st.clone();
        if old.status != new.status { p.playback_status_changed(&ctxt).await?; }
        if (old.title, old.artist, old.album, old.art_url, old.track, old.url) != (new.title.clone(), new.artist.clone(), new.album.clone(), new.art_url.clone(), new.track, new.url.clone())
            || (old.length - new.length).abs() > 0.5 {
            p.metadata_changed(&ctxt).await?;
            p.can_seek_changed(&ctxt).await?;
        }
        if old.can_next != new.can_next { p.can_go_next_changed(&ctxt).await?; }
        if old.can_prev != new.can_prev { p.can_go_previous_changed(&ctxt).await?; }
        if (old.rate - new.rate).abs() > 1e-6 { p.rate_changed(&ctxt).await?; }
        if (old.volume - new.volume).abs() > 1e-6 { p.volume_changed(&ctxt).await?; }
        if new.seeked { Player::seeked(&ctxt, (new.position * 1e6) as i64).await?; }
        Ok(())
    }
}
