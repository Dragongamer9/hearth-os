//! Hearth Terminal's window-free parts: the shell's pseudo-terminal (a Unix pty on Linux, ConPTY on Windows),
//! the list of shells, settings, the command line, and the on-screen keyboard. main.rs puts them behind the
//! terminal page (ui/index.html, xterm.js).

/// Safe settings files shared by Hearth's apps (atomic, .bak, damaged files put back, never defaults over them).
#[path = "../../hearth-apptheme/appfile.rs"]
pub mod appfile;
pub mod cli;
pub mod keyboard;
pub mod pty;
pub mod settings;
pub mod shells;
pub mod utf8;
