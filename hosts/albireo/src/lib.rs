//! Hearth's own Wayland compositor (docs/ALBIREO.md). Started from Smithay's Anvil (MIT; LICENSE-SMITHAY-ANVIL.txt):
//! its backends, shells, input and rendering, with Hearth's parts added: window frames in Mantel's colours
//! (shell/ssd.rs, theme.rs), the taskbar protocol Mantel uses on labwc (foreign_toplevel.rs), Mantel's shortcuts
//! from rc.xml (keybinds.rs), minimizing, snapping and the rest of the window management (wm.rs).
#![warn(rust_2018_idioms)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(any(feature = "winit", feature = "udev")), allow(dead_code, unused_imports))]

#[cfg(any(feature = "udev", feature = "xwayland"))]
pub mod cursor;
pub mod drawing;
pub mod focus;
pub mod foreign_toplevel;
pub mod input_handler;
pub mod keybinds;
pub mod options;
pub mod render;
pub mod shell;
pub mod state;
pub mod theme;
#[cfg(feature = "udev")]
pub mod udev;
#[cfg(feature = "winit")]
pub mod winit;
pub mod wm;

pub use state::{ClientState, HearthState};
