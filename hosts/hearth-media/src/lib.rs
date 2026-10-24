//! Hearth's media apps share this: Hearth Recorder (hosts/hearth-recorder), Hearth Camera (hosts/hearth-camera)
//! and Hearth Share (hosts/hearth-share). Nothing in here has a window; it finds and runs the tools that do the
//! real work (ffmpeg, wf-recorder, gpu-screen-recorder, GStreamer, pactl, notify-send) and moves their output around.
//!  - tools: finding and running programs, signals, detached processes
//!  - dirs: Videos / Pictures / Music (xdg user dirs), runtime, cache and config folders, file names with the time
//!  - env: which kind of session this is (Game Mode's gamescope, Mantel's wlroots Wayland, X11)
//!  - encoders: which video encoders really work here (NVENC, VA-API, software) and their arguments
//!  - pulse: the speakers' and the microphone's sources, and a mix of both
//!  - jpeg, hub, http: JPEG frames out of a pipe, shared with any number of viewers over HTTP
//!  - notify: "Saved" notifications with Open and Show in folder
//!  - files: lists of recordings, photos and videos

pub mod dirs;
pub mod encoders;
pub mod env;
pub mod files;
pub mod http;
pub mod hub;
pub mod jpeg;
pub mod notify;
pub mod pulse;
pub mod tools;
