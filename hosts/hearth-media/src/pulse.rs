//! Sound sources through PipeWire's PulseAudio side (pactl): what's playing (the speakers' monitor), the
//! microphone, and both at once (a private mix: a null sink with a loopback from each, taken away afterwards).

use crate::tools;
use std::time::Duration;

fn pactl(args: &[&str]) -> Option<String> {
    if !tools::have("pactl") { return None; }
    let o = tools::run("pactl", args, Duration::from_secs(5)).ok()?;
    if o.ok() { Some(o.stdout.trim().to_string()) } else { None }
}

pub fn default_sink() -> Option<String> { pactl(&["get-default-sink"]).filter(|s| !s.is_empty()) }
pub fn default_source() -> Option<String> { pactl(&["get-default-source"]).filter(|s| !s.is_empty()) }

/// What's playing: the default output's monitor.
pub fn system_source() -> String {
    default_sink().map(|s| format!("{s}.monitor")).unwrap_or_else(|| "@DEFAULT_MONITOR@".into())
}

/// The microphone, when there is one (a default source that's only a monitor means there isn't).
pub fn mic_source() -> Option<String> {
    match default_source() {
        Some(s) if s.ends_with(".monitor") => None,
        Some(s) => Some(s),
        None => if tools::have("pactl") { None } else { Some("@DEFAULT_SOURCE@".into()) },
    }
}

/// A private mix of several sources (taken away again when dropped).
pub struct Mix { modules: Vec<String>, pub source: String }

impl Mix {
    pub fn new(sources: &[String]) -> Result<Mix, String> {
        let name = format!("hearth_mix_{}", std::process::id());
        let before = default_sink();
        let sink = pactl(&["load-module", "module-null-sink", &format!("sink_name={name}"), "sink_properties=device.description=Hearth-recording-mix"])
            .ok_or("Couldn't mix the sounds (pactl load-module failed)")?;
        let mut m = Mix { modules: vec![sink], source: format!("{name}.monitor") };
        for s in sources {
            match pactl(&["load-module", "module-loopback", &format!("source={s}"), &format!("sink={name}"), "latency_msec=20", "source_dont_move=true", "sink_dont_move=true"]) {
                Some(id) => m.modules.push(id),
                None => return Err(format!("Couldn't add {s} to the mix")),
            }
        }
        // A new sink must never become where everything plays.
        if let Some(b) = before { if default_sink().as_deref() != Some(b.as_str()) { pactl(&["set-default-sink", &b]); } }
        Ok(m)
    }
    pub fn close(&mut self) {
        for id in self.modules.drain(..).rev() { pactl(&["unload-module", &id]); }
    }
}

impl Drop for Mix {
    fn drop(&mut self) { self.close(); }
}

/// The one source to record from for "system sound" and/or "microphone": None for silence, plus the mix to
/// keep alive while recording when it's both. Err when the microphone is wanted and there's none.
pub fn source_for(system: bool, mic: bool) -> Result<(Option<String>, Option<Mix>), String> {
    match (system, mic) {
        (false, false) => Ok((None, None)),
        (true, false) => Ok((Some(system_source()), None)),
        (false, true) => mic_source().map(|m| (Some(m), None)).ok_or_else(|| "There's no microphone".to_string()),
        (true, true) => {
            let Some(m) = mic_source() else { return Ok((Some(system_source()), None)) };
            let mix = Mix::new(&[system_source(), m])?;
            Ok((Some(mix.source.clone()), Some(mix)))
        }
    }
}
