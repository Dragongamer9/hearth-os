//! hearth-gamma: Desktop Mode's colour filter (Settings › Accessibility, desktop/shell/hearth-access starts and stops
//! it). labwc has no colour shaders, so this is what a gamma ramp can do: it asks labwc for every screen's colour
//! ramp (the wlr-gamma-control protocol) and holds an inverted one until it's stopped; labwc puts the screens back
//! the moment it ends. Only one program can hold a screen's ramp, so Night light (wlsunset) has to be off.
//!
//!   hearth-gamma invert      invert the colours until stopped (SIGTERM, or pkill -x hearth-gamma)
//!   hearth-gamma check       print the screens and their ramp sizes, then end (for testing)

use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsFd, FromRawFd, OwnedFd};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_output, wl_registry};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1;
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_v1::{self, ZwlrGammaControlV1};

#[derive(Clone, Copy, PartialEq)]
enum Mode { Invert, Check }

struct State {
    mode: Mode,
    mgr: ZwlrGammaControlManagerV1,
    screens: usize,
    answered: usize,
    failed: usize,
    // The ramps sent (labwc keeps its own copy; kept here until the end anyway).
    ramps: Vec<OwnedFd>,
}

/// One channel's ramp of `size` steps: inverted (bright to dark) or the usual (dark to bright).
fn ramp(size: usize, invert: bool) -> Vec<u16> {
    (0..size).map(|i| {
        let v = if size > 1 { i as f64 / (size - 1) as f64 } else { 1.0 };
        let v = if invert { 1.0 - v } else { v };
        (v * 65535.0).round() as u16
    }).collect()
}

/// The three channels' ramps one after the other (red, green, blue), as labwc reads them, in a memory file.
fn ramp_file(size: usize, invert: bool) -> std::io::Result<OwnedFd> {
    let fd = unsafe { libc::memfd_create(b"hearth-gamma\0".as_ptr().cast(), libc::MFD_CLOEXEC) };
    if fd < 0 { return Err(std::io::Error::last_os_error()); }
    let mut f = unsafe { std::fs::File::from_raw_fd(fd) };
    let one: Vec<u8> = ramp(size, invert).iter().flat_map(|v| v.to_ne_bytes()).collect();
    for _ in 0..3 { f.write_all(&one)?; }
    f.seek(SeekFrom::Start(0))?;
    Ok(OwnedFd::from(f))
}

impl State {
    fn add_screen(&mut self, out: &wl_output::WlOutput, qh: &QueueHandle<Self>) {
        let n = self.screens;
        self.screens += 1;
        self.mgr.get_gamma_control(out, qh, n);
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(state: &mut Self, reg: &wl_registry::WlRegistry, event: wl_registry::Event, _: &GlobalListContents, _: &Connection, qh: &QueueHandle<Self>) {
        // A screen plugged in while it runs gets the filter too.
        if let wl_registry::Event::Global { name, interface, version } = event {
            if interface == "wl_output" && state.mode == Mode::Invert {
                let out: wl_output::WlOutput = reg.bind(name, version.min(4), qh, ());
                state.add_screen(&out, qh);
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(_: &mut Self, _: &wl_output::WlOutput, _: wl_output::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrGammaControlManagerV1, ()> for State {
    fn event(_: &mut Self, _: &ZwlrGammaControlManagerV1, _: <ZwlrGammaControlManagerV1 as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ZwlrGammaControlV1, usize> for State {
    fn event(state: &mut Self, ctl: &ZwlrGammaControlV1, event: zwlr_gamma_control_v1::Event, n: &usize, _: &Connection, _: &QueueHandle<Self>) {
        match event {
            zwlr_gamma_control_v1::Event::GammaSize { size } => {
                state.answered += 1;
                if state.mode == Mode::Check { println!("screen {n}: ramp of {size} steps"); return; }
                match ramp_file(size as usize, true) {
                    Ok(fd) => { ctl.set_gamma(fd.as_fd()); state.ramps.push(fd); }
                    Err(e) => eprintln!("screen {n}: {e}"),
                }
            }
            zwlr_gamma_control_v1::Event::Failed => {
                state.answered += 1;
                state.failed += 1;
                eprintln!("screen {n}: its colour ramp is in use by another program (Night light?) or can't be changed");
                ctl.destroy();
            }
            _ => {}
        }
    }
}

fn main() {
    let mode = match std::env::args().nth(1).as_deref() {
        Some("invert") => Mode::Invert,
        Some("check") => Mode::Check,
        _ => { eprintln!("usage: hearth-gamma invert | check"); std::process::exit(2); }
    };
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => { eprintln!("no Wayland display: {e}"); std::process::exit(1); }
    };
    let (globals, mut queue) = match registry_queue_init::<State>(&conn) {
        Ok(g) => g,
        Err(e) => { eprintln!("{e}"); std::process::exit(1); }
    };
    let qh = queue.handle();
    let mgr: ZwlrGammaControlManagerV1 = match globals.bind(&qh, 1..=1, ()) {
        Ok(m) => m,
        Err(_) => { eprintln!("this compositor doesn't let programs change the colour ramp (wlr-gamma-control)"); std::process::exit(1); }
    };
    let mut state = State { mode, mgr, screens: 0, answered: 0, failed: 0, ramps: Vec::new() };
    for g in globals.contents().clone_list() {
        if g.interface == "wl_output" {
            let out: wl_output::WlOutput = globals.registry().bind(g.name, g.version.min(4), &qh, ());
            state.add_screen(&out, &qh);
        }
    }
    if state.screens == 0 { eprintln!("no screens"); std::process::exit(1); }
    // Every screen answers first (its ramp size, or that it can't).
    while state.answered < state.screens {
        if let Err(e) = queue.blocking_dispatch(&mut state) { eprintln!("{e}"); std::process::exit(1); }
    }
    if state.failed == state.screens { std::process::exit(1); }
    if mode == Mode::Check { return; }
    // Then hold the ramps until stopped: labwc puts the screens back when this ends.
    loop {
        if let Err(e) = queue.blocking_dispatch(&mut state) { eprintln!("{e}"); std::process::exit(1); }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ramps() {
        assert_eq!(super::ramp(3, false), vec![0, 32768, 65535]);
        assert_eq!(super::ramp(3, true), vec![65535, 32768, 0]);
        assert_eq!(super::ramp(1, true), vec![0]);
    }
    #[test]
    fn ramp_file_holds_three_channels() {
        use std::io::Read;
        let fd = super::ramp_file(256, true).unwrap();
        let mut f = std::fs::File::from(fd);
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        assert_eq!(b.len(), 256 * 2 * 3);
        assert_eq!(u16::from_ne_bytes([b[0], b[1]]), 65535);
    }
}
