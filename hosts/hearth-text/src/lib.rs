//! Hearth Text's window-free parts: reading and saving files (encodings, line endings, big files, saving
//! safely), saving as administrator on Linux, settings and the command line. main.rs puts them behind the editor
//! page (ui/index.html, CodeMirror 6).

/// Safe settings files shared by Hearth's apps (atomic, .bak, damaged files put back, never defaults over them).
#[path = "../../hearth-apptheme/appfile.rs"]
pub mod appfile;
pub mod admin;
pub mod cli;
pub mod files;
pub mod settings;
pub mod textfile;
