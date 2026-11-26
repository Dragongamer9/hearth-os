//! Going live to Twitch and YouTube at once (board 12, Where): OBS streams to one place at a time, so with two or
//! more places Hearth Studio runs its own relay: OBS streams to it on this PC (rtmp://127.0.0.1:19350/hearth, set as
//! OBS's custom server while you're live), and ffmpeg copies that one stream, untouched (no second encode), to every
//! place you picked (its tee muxer; a place that drops out doesn't stop the others). When you end the stream the relay
//! stops and OBS is set back to your first place, so OBS on its own still goes there.
//! Stream keys: you type each into Hearth Studio once; it keeps them in your keyring (secret-tool, the key on its
//! stdin), never in a file or a log, and hands them to the relay only while you're live.

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub const LISTEN: &str = "rtmp://127.0.0.1:19350/hearth";
pub const KEY: &str = "live";
pub const PLACES: [&str; 3] = ["Twitch", "YouTube", "Kick"];

static RELAY: Mutex<Option<Child>> = Mutex::new(None);

/// Where a place takes a stream, with its key.
pub fn ingest(place: &str, key: &str) -> String {
    match place {
        "YouTube" => format!("rtmps://a.rtmps.youtube.com:443/live2/{key}"),
        "Kick" => format!("rtmps://fa723fc1b171.global-contribute.live-video.net:443/app/{key}"),
        _ => format!("rtmp://live.twitch.tv/app/{key}"),
    }
}

/// OBS's own name for a place's service (rtmp_common).
pub fn service(place: &str) -> (&'static str, &'static str) {
    match place {
        "YouTube" => ("YouTube - RTMPS", "rtmps://a.rtmps.youtube.com:443/live2"),
        "Kick" => ("Kick", "auto"),
        _ => ("Twitch", "auto"),
    }
}

/// ffmpeg's words for the relay: listen for OBS, copy to each place.
pub fn args(targets: &[(String, String)]) -> Vec<String> {
    let tee = targets.iter().map(|(p, k)| format!("[f=flv:onfail=ignore]{}", ingest(p, k).replace('|', "%7C"))).collect::<Vec<_>>().join("|");
    ["-hide_banner", "-loglevel", "error", "-listen", "1", "-i", &format!("{LISTEN}/{KEY}"), "-c", "copy", "-map", "0", "-f", "tee", &tee].iter().map(|s| s.to_string()).collect()
}

pub fn running() -> bool {
    RELAY.lock().ok().map(|mut g| g.as_mut().map(|c| c.try_wait().ok().flatten().is_none()).unwrap_or(false)).unwrap_or(false)
}

/// The relay started (before OBS starts streaming to it). `targets`: (place, key).
pub fn start(targets: &[(String, String)]) -> Result<(), String> {
    stop();
    let ff = std::env::var("HEARTH_STUDIO_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let c = Command::new(ff).args(args(targets)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("ffmpeg couldn't start: {e}"))?;
    if let Ok(mut g) = RELAY.lock() {
        *g = Some(c);
    }
    // a moment for it to listen
    std::thread::sleep(std::time::Duration::from_millis(600));
    if running() { Ok(()) } else { Err("the relay stopped as it started (is ffmpeg here?)".into()) }
}

pub fn stop() {
    if let Ok(mut g) = RELAY.lock() {
        if let Some(mut c) = g.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_stream_to_every_place() {
        let a = args(&[("Twitch".into(), "tw".into()), ("YouTube".into(), "yt".into())]);
        assert!(a.contains(&"rtmp://127.0.0.1:19350/hearth/live".to_string()));
        assert_eq!(a.last().unwrap(), "[f=flv:onfail=ignore]rtmp://live.twitch.tv/app/tw|[f=flv:onfail=ignore]rtmps://a.rtmps.youtube.com:443/live2/yt");
        assert!(a.iter().any(|x| x == "copy"));
        assert_eq!(service("YouTube").0, "YouTube - RTMPS");
    }
}
