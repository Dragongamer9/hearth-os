//! What this PC can record with: the kind of session, the capture tools that are installed, and the encoders
//! that really work (hearth-media's encoders.rs tries them).
//! Tests and the owner can steer it: HEARTH_RECORDER_BACKEND=gsr|wf|x11|gst forces one way, and
//! HEARTH_RECORDER_NO_GSR=1 pretends gpu-screen-recorder isn't there.

use hearth_media::encoders::{self, Encoder};
use hearth_media::env::{self, Session};
use hearth_media::tools;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub session: Session,
    /// X11's $DISPLAY (for ffmpeg's x11grab)
    pub display: String,
    pub ffmpeg: bool,
    /// wf-recorder (wlroots: Mantel's labwc)
    pub wf: bool,
    /// gpu-screen-recorder
    pub gsr: bool,
    /// gpu-screen-recorder can record an area (-w region)
    pub gsr_region: bool,
    pub gst: bool,
    /// GStreamer's pipewiresrc (Game Mode's gamescope stream)
    pub gst_pipewire: bool,
    pub slurp: bool,
    pub pactl: bool,
    pub pw_record: bool,
    /// gamescope's PipeWire stream, when it has one
    pub gamescope_node: Option<u32>,
    pub encoders: Vec<Encoder>,
    pub audio_encoders: Vec<String>,
    pub gst_encoders: Vec<Encoder>,
    pub gst_audio: Option<String>,
    pub render: String,
    pub vendors: Vec<String>,
    /// HEARTH_RECORDER_BACKEND
    pub force: Option<String>,
}

impl Env {
    /// Nothing at all (tests build on this).
    pub fn empty(session: Session) -> Env {
        Env { session, display: String::new(), ffmpeg: false, wf: false, gsr: false, gsr_region: false, gst: false, gst_pipewire: false,
              slurp: false, pactl: false, pw_record: false, gamescope_node: None, encoders: vec![], audio_encoders: vec![],
              gst_encoders: vec![], gst_audio: None, render: String::new(), vendors: vec![], force: None }
    }

    /// Looks at this PC. `fresh` tries the encoders again instead of using what's remembered.
    pub fn detect(fresh: bool) -> Env {
        let session = env::session();
        let mut e = Env::empty(session);
        e.display = std::env::var("DISPLAY").unwrap_or_default();
        e.ffmpeg = tools::have("ffmpeg");
        e.wf = tools::have("wf-recorder");
        e.gsr = tools::have("gpu-screen-recorder") && std::env::var("HEARTH_RECORDER_NO_GSR").map(|v| v.is_empty() || v == "0").unwrap_or(true);
        if e.gsr {
            let help = tools::run("gpu-screen-recorder", &["--help"], Duration::from_secs(5)).map(|o| format!("{}{}", o.stdout, o.stderr)).unwrap_or_default();
            let version = tools::run("gpu-screen-recorder", &["--version"], Duration::from_secs(5)).map(|o| format!("{}{}", o.stdout, o.stderr)).unwrap_or_default();
            e.gsr_region = gsr_has_region(&help, &version);
        }
        e.slurp = tools::have("slurp");
        e.pactl = tools::have("pactl");
        e.pw_record = tools::have("pw-record");
        e.render = encoders::render_node().unwrap_or_default();
        e.vendors = encoders::vendors().into_iter().map(String::from).collect();
        if e.ffmpeg { let (v, a) = encoders::ffmpeg(fresh); e.encoders = v; e.audio_encoders = a; }
        e.gst = tools::have("gst-launch-1.0");
        // GStreamer is only for Game Mode (gamescope's stream); trying its encoders takes a moment.
        if session == Session::Gamescope && e.gst {
            e.gst_pipewire = encoders::gst_has("pipewiresrc");
            if e.gst_pipewire {
                e.gamescope_node = gamescope_node();
                e.gst_encoders = encoders::gstreamer();
                e.gst_audio = encoders::gst_audio_enc().map(String::from);
            }
        }
        e.force = std::env::var("HEARTH_RECORDER_BACKEND").ok().filter(|v| !v.is_empty());
        e
    }
}

/// Whether gpu-screen-recorder records an area (-w region). Its help says so; when the help can't be read (it
/// can refuse to start without a screen, or print only a short usage line), its version does: 5.10 and later have it.
pub fn gsr_has_region(help: &str, version: &str) -> bool {
    if help.contains("region") { return true; }
    // The first word that is a version (5.13.8): it can print warnings with paths (/root/.cache) first.
    let v = version.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map(|w| w.trim_matches('.'))
        .find(|w| { let p: Vec<&str> = w.split('.').collect(); p.len() >= 2 && p.iter().all(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit())) })
        .unwrap_or("");
    let mut n = v.split('.').map(|x| x.parse::<u32>().unwrap_or(0));
    let (major, minor) = (n.next().unwrap_or(0), n.next().unwrap_or(0));
    major > 5 || (major == 5 && minor >= 10)
}

/// gamescope's PipeWire video stream (the node called "gamescope"), from pw-dump.
pub fn gamescope_node() -> Option<u32> {
    let o = tools::run("pw-dump", &[], Duration::from_secs(5)).ok()?;
    parse_node(&o.stdout, "gamescope")
}

pub fn parse_node(dump: &str, name: &str) -> Option<u32> {
    let v: Value = serde_json::from_str(dump).ok()?;
    v.as_array()?.iter().find(|o| {
        o["type"] == "PipeWire:Interface:Node" && o["info"]["props"]["node.name"] == name
            && o["info"]["props"]["media.class"].as_str().map(|c| c.starts_with("Video")).unwrap_or(true)
    }).and_then(|o| o["id"].as_u64()).map(|n| n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_gamescope() {
        let d = r#"[{"id": 31, "type": "PipeWire:Interface:Node", "info": {"props": {"node.name": "alsa_output.pci", "media.class": "Audio/Sink"}}},
                    {"id": 77, "type": "PipeWire:Interface:Node", "info": {"props": {"node.name": "gamescope", "media.class": "Video/Source"}}}]"#;
        assert_eq!(parse_node(d, "gamescope"), Some(77));
        assert_eq!(parse_node(d, "nope"), None);
        assert_eq!(parse_node("not json", "gamescope"), None);
    }
    #[test]
    fn gsr_region_from_help_or_version() {
        assert!(gsr_has_region("-w <window_id|monitor|focused|portal|region|v4l2_device_path>", ""));
        assert!(gsr_has_region("", "5.13.8\n"));
        assert!(gsr_has_region("usage: gpu-screen-recorder -w", "gpu-screen-recorder 6.1.5"));
        assert!(!gsr_has_region("", "5.3.0"));
        assert!(gsr_has_region("", "gsr warning: install_cuda_no_stable_perf_limit: failed to create directory: /root/.cache/x.1\n5.13.8\n"));
        assert!(!gsr_has_region("", "gsr warning: failed to create directory: /root/.cache"));
        assert!(!gsr_has_region("", ""));
    }
}
