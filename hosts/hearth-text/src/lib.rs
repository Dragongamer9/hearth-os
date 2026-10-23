//! Hearth Text's window-free parts: reading and saving files (encodings, line endings, big files, saving
//! safely), saving as administrator on Linux, settings and the command line. main.rs puts them behind the editor
//! page (ui/index.html, CodeMirror 6).

pub mod admin;
pub mod cli;
pub mod files;
pub mod settings;
pub mod textfile;
